//! The left panel's Agent tab, beside Model: a chat with an AI agent that edits the open Forge
//! file, Cursor's agent panel in the browser's place. This keeps and draws the transcript; the
//! host runs the backend (`crate::agent`) and does what `AgentAct`s ask.

use super::agent_settings::{self, AgentSettings, SettingsAct};
use super::theme::{Theme, medium, mono, sans, semi};
use super::{HEAD_H, Shell, icons};
use egui::text::{LayoutJob, TextFormat};
use egui::{Align2, Color32, FontId, Key, Modifiers, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};
use forge_agent::json::Json;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SideTab {
    #[default]
    Model,
    Agent,
    Versions,
}

/// What a backend says, in order, from its thread.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// The agent's reply, a piece at a time.
    Text(String),
    ToolStart {
        id: String,
        name: String,
    },
    /// What the tool was asked: a line for its row, the whole for its opened row.
    ToolInput {
        id: String,
        summary: String,
        input: String,
    },
    ToolDone {
        id: String,
        output: String,
        error: bool,
    },
    /// Said in the transcript, not by the agent: a refused model, a dropped connection.
    Note {
        text: String,
        error: bool,
    },
    /// The turn's over: finished, stopped, or failed with this.
    Done {
        error: Option<String>,
        stopped: bool,
    },
    /// The backend can't run at all: why, and the one command that fixes it.
    Problem {
        text: String,
        fix: Option<String>,
    },
    /// The agent asks before a tool runs: `id` answers it (`Backend::answer`). `summary` is the
    /// tool's row ("Edit plate.fg"), `input` the whole of what it would do; `always`, when the
    /// backend offers it, what "Always" would allow ("Accept edits for this chat").
    Permission {
        id: String,
        tool: String,
        /// The tool call it's about: its row says it waits.
        tool_use: String,
        summary: String,
        input: String,
        always: Option<String>,
        /// The tool's input as sent (JSON): the host reads an edit's file to show it whole.
        raw: String,
    },
    /// The backend's permission mode is now this (`Mode::flag`), however it changed.
    Mode(String),
    /// The slash commands the backend takes in a message (Claude Code's `init` says them).
    Commands(Vec<String>),
    /// Signed in with an API key (paid per token), or not (a Claude plan).
    ApiKeyed(bool),
    /// The plan's limit: reached, or close to it (else neither: allowed), and when it resets
    /// (seconds since the epoch).
    Limit {
        reached: bool,
        close: bool,
        resets: Option<f64>,
    },
    /// What a turn took: seconds, tokens in and out, and what the API would charge for them.
    Usage {
        secs: f64,
        tokens_in: u64,
        tokens_out: u64,
        cost: f64,
    },
    /// A Forge file in the folder changed or appeared while the agent ran; what it held before.
    FileChanged {
        path: String,
        before: Option<String>,
        text: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Line {
    Same(String),
    Add(String),
    Del(String),
    /// Unchanged lines left out.
    Skip(usize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolState {
    Running,
    Done,
    Failed,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Entry {
    User {
        text: String,
        refs: Vec<String>,
        /// What was drawn for it, once the host has sent it.
        markup: Option<super::markup::Sent>,
        /// How many pictures were pasted or dropped with it.
        pictures: usize,
    },
    Agent {
        text: String,
    },
    Tool {
        id: String,
        name: String,
        summary: String,
        input: String,
        output: String,
        state: ToolState,
    },
    /// A turn's change to one file, as one undo step; `created`: the agent made the file.
    Diff {
        turn: usize,
        index: usize,
        created: bool,
        file: String,
        lines: Vec<Line>,
        added: usize,
        removed: usize,
        undone: bool,
    },
    Note {
        text: String,
        error: bool,
    },
    /// A tool waiting on the person, and then what they said.
    Permission {
        id: String,
        tool: String,
        tool_use: String,
        summary: String,
        input: String,
        always: Option<String>,
        state: Asked,
    },
}

/// A permission's answer, or none yet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Asked {
    Waiting,
    Allowed,
    /// Allowed, and what `always` names from now on.
    Always,
    Denied,
    /// The turn ended (stopped, failed) before an answer.
    Dropped,
}

/// How a permission is answered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Answer {
    Allow,
    Always,
    Deny,
}

/// What may run without asking, as Claude Code's `--permission-mode` names it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    /// Ask before edits and commands the person's settings don't already allow.
    #[default]
    Ask,
    /// Edit files without asking; ask before the rest.
    AcceptEdits,
    /// Look and plan, change nothing.
    Plan,
}

impl Mode {
    pub const ALL: [Mode; 3] = [Mode::Ask, Mode::AcceptEdits, Mode::Plan];
    pub fn flag(self) -> &'static str {
        match self {
            Mode::Ask => "default",
            Mode::AcceptEdits => "acceptEdits",
            Mode::Plan => "plan",
        }
    }
    pub fn from_flag(s: &str) -> Option<Mode> {
        Mode::ALL.into_iter().find(|m| m.flag() == s)
    }
    pub fn word(self) -> &'static str {
        match self {
            Mode::Ask => "Ask first",
            Mode::AcceptEdits => "Accept edits",
            Mode::Plan => "Plan only",
        }
    }
    pub fn tip(self) -> &'static str {
        match self {
            Mode::Ask => "The agent asks before it edits a file or runs a command (Shift+Tab to change)",
            Mode::AcceptEdits => {
                "The agent edits files without asking; it still asks before commands (Shift+Tab to change)"
            }
            Mode::Plan => "The agent reads and plans but changes nothing (Shift+Tab to change)",
        }
    }
    /// What it means, for the menu.
    pub fn means(self) -> &'static str {
        match self {
            Mode::Ask => "Asks before it edits a file or runs a command",
            Mode::AcceptEdits => "Edits files without asking; asks before commands",
            Mode::Plan => "Reads and plans; changes nothing",
        }
    }

    pub fn next(self) -> Mode {
        match self {
            Mode::Ask => Mode::AcceptEdits,
            Mode::AcceptEdits => Mode::Plan,
            Mode::Plan => Mode::Ask,
        }
    }
}

/// What the tab asks of the host.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentAct {
    /// `markup`: the marks go too, the host sending once the frame with them is drawn.
    Send {
        text: String,
        refs: Vec<String>,
        markup: bool,
        /// Pasted or dropped, after the view's picture if there's one.
        pictures: Vec<crate::agent::Picture>,
        /// Its card is in the chat already; else the host adds it when the message goes.
        shown: bool,
    },
    /// A sent message's marks shown again: its entry, the camera they were drawn over put back.
    ShowMarkup(usize),
    /// Remember this device with the bridge for a week (true), or forget it.
    RememberDevice(bool),
    /// A queued message: dropped, or (`edit`) taken back into the box.
    Unqueue {
        index: usize,
        edit: bool,
    },
    Stop,
    NewChat,
    ListConversations,
    ResumeConversation(String),
    /// A turn's change taken back, or put back.
    Undo(usize, usize),
    Redo(usize, usize),
    Settings(SettingsAct),
    /// A permission answered.
    Answer(String, Answer),
    /// The permission mode changed in the tab.
    Mode(Mode),
    /// Back to the person's own settings' mode (its flag): `AgentTab::own_mode`.
    OwnMode(String),
    /// The answer to "Trust this folder for the agent?".
    Trust {
        dir: String,
        yes: bool,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Problem {
    pub text: String,
    pub fix: Option<String>,
}

#[derive(Default)]
pub struct AgentTab {
    pub tab: SideTab,
    pub entries: Vec<Entry>,
    pub input: String,
    /// Ids attached to the next message: a feature, face or occurrence.
    pub refs: Vec<String>,
    pub running: bool,
    pub queued: usize,
    /// The queued messages' words, in the order they'll go.
    pub queue: Vec<String>,
    /// Put the keyboard in the box next frame (a queued message taken back to edit).
    pub focus_input: bool,
    reply_boundary: bool,
    pub problem: Option<Problem>,
    /// The open file's name, for the context line.
    pub file: Option<String>,
    pub project: Option<String>,
    pub settings: AgentSettings,
    pub switcher: bool,
    pub resume_open: bool,
    pub resume_filter: String,
    pub resume_error: Option<String>,
    pub saved_chats: Vec<crate::agent::conversations::Summary>,
    pub out: Vec<AgentAct>,
    /// Tool rows opened to show what they ran.
    pub open: std::collections::HashSet<String>,
    /// What the selection is, as the agent names it, if anything's selected.
    pub selection: Option<String>,
    /// Everything selected, when it's more than one: offered as one chip, attached together.
    pub selection_all: Vec<String>,
    /// A folder outside the projects tree, asked about once; the message waiting on it.
    pub trust_ask: Option<String>,
    pub pending: Option<(String, Vec<String>, Vec<crate::agent::Picture>)>,
    pub markup: super::markup::Markup,
    /// Each sent markup's thumbnail, by its entry.
    thumbs: std::collections::HashMap<usize, egui::TextureHandle>,
    /// Talking instead of typing (`voice.rs`).
    pub voice: super::voice::Voice,
    /// What may run without asking, this chat, as the backend last said or the person chose.
    pub mode: Mode,
    /// The person chose `mode` here (else it's their own settings' default).
    pub mode_chosen: bool,
    /// The mode chip's menu is open.
    pub mode_menu: bool,
    /// The backend's own slash commands (Claude Code's: compact, context, skills...).
    pub commands: Vec<String>,
    /// The last turn's time, tokens in and out, and API-equivalent cost, said under it.
    pub usage: Option<(f64, u64, u64, f64)>,
    /// When the running turn began (the frame clock's seconds), for its time so far.
    pub turn_started: Option<f64>,
    /// ↑ in the box: which earlier message it holds now, counted back from the last.
    pub recall: Option<usize>,
    /// Scroll the chat to its end next frame ("Jump to latest").
    pub jump: bool,
    /// The waiting question's buttons are in sight (or none waits): Enter may answer it.
    pub card_visible: bool,
    /// A mode the backend said that the tab has no name for (`bypassPermissions`, `auto`): shown
    /// as it is, instead of `mode`.
    pub mode_other: Option<String>,
    /// The mode the person's own settings gave the chat at its start, when the tab has no name
    /// for it (a bypass, `auto`): the mode menu offers it back after another is chosen.
    pub own_mode: Option<String>,
    /// A chat brought back: where it was resumed (the entries before it are the old ones) and
    /// what the divider there says ("Resumed · 2 h ago").
    pub resumed: Option<(usize, String)>,
    /// Paid per token (an API key): a turn's cost is said. A plan's turns aren't charged.
    pub api_keyed: bool,
    /// `/new` just started this chat: the empty chat says where the last one went.
    pub started_new: bool,
    /// A picture of the view goes with the next message (the input's view button).
    pub attach_view: bool,
    /// In a page paired with a bridge: Some(0) by the link, else Some(when this remembered
    /// device lapses). None elsewhere (no row for it).
    pub device: Option<u64>,
    /// Pictures pasted or dropped for the next message, in order.
    pub pictures: Vec<crate::agent::Picture>,
    /// The chat's entries' heights as last drawn, and the width they were drawn at.
    heights: (f32, Vec<f32>),
    /// The plan's limit, when it's reached or close: (reached, when it resets).
    pub limit: Option<(bool, Option<f64>)>,
    /// A selection the person took off the next message (its chip's ×): not suggested again
    /// until something else is selected.
    pub dismissed: Option<String>,
    /// The `@` list under the cursor's word, while one is open.
    pub mention: Option<Mention>,
    /// The box as it was when Esc closed the `@` list: it stays closed until the text changes.
    pub mention_off: Option<String>,
    /// The id of the chip the pointer is on (in the box or a sent message): the view lights what
    /// it names. Set each frame.
    pub hover_ref: Option<String>,
}

/// One thing `@` can name: what goes in the message, and what it is.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Candidate {
    pub name: String,
    pub what: String,
    /// More, on the row's hover: what a param drives.
    pub tip: String,
    /// Whose command it is (`/`'s list): "app", "Claude Code" or "skill"; empty for `@`'s.
    pub source: &'static str,
}

/// The `@` list: the word typed after `@`, what matches it, and the one Enter takes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Mention {
    /// `@` (a name) or `/` (a command).
    pub sigil: char,
    pub word: String,
    pub items: Vec<Candidate>,
    pub at: usize,
    /// Everything it can name, gathered once when the list opened.
    pub all: Vec<Candidate>,
}

/// The `/word` the box is, when it's a command being typed: the word, without its `/`.
pub fn slash_word(input: &str) -> Option<&str> {
    let w = input.strip_prefix('/')?;
    w.chars()
        .all(|c| c.is_alphanumeric() || "-_:".contains(c))
        .then_some(w)
}

/// The app's own commands, before the backend's: (name, what it does).
pub const COMMANDS: [(&str, &str); 7] = [
    ("new", "Start a new conversation"),
    ("clear", "Start a new conversation (as /new)"),
    ("resume", "Pick a saved conversation"),
    ("mode", "What may run without asking: ask, edits or plan"),
    ("model", "Choose the agent and its model"),
    ("undo", "Take back the last turn's changes"),
    ("help", "What these commands do"),
];

/// What a backend command does, where it's known.
fn backend_says(name: &str) -> String {
    match name {
        "compact" => "Summarise the conversation so far, to free context".into(),
        "context" => "What's using the context".into(),
        "usage" => "Your Claude plan's usage".into(),
        "init" => "Write a CLAUDE.md for this project".into(),
        "effort" => "How hard the model thinks".into(),
        "review" | "code-review" => "Review the changes".into(),
        "security-review" => "Review the changes for security".into(),
        n if n.contains(':') => "A skill".into(),
        _ => String::new(),
    }
}

/// Everything `/` can run: the app's, then the backend's (less those the app does itself).
pub fn command_candidates(backend: &[String]) -> Vec<Candidate> {
    let mut all: Vec<Candidate> = COMMANDS
        .iter()
        .map(|(n, w)| Candidate {
            name: n.to_string(),
            what: w.to_string(),
            tip: String::new(),
            source: "app",
        })
        .collect();
    for n in backend {
        if n.starts_with("__") || COMMANDS.iter().any(|c| c.0 == n) {
            continue;
        }
        all.push(Candidate {
            name: n.clone(),
            what: backend_says(n),
            tip: String::new(),
            source: if n.contains(':') { "skill" } else { "Claude Code" },
        });
    }
    all
}

/// At most this many in the `@` list.
pub const MENTION_ROWS: usize = 8;

/// The `@word` the box ends with, if it does: the word, without its `@`.
pub fn mention_word(input: &str) -> Option<&str> {
    let start = input.rfind(|c: char| c.is_whitespace()).map_or(0, |k| k + 1);
    let word = input[start..].strip_prefix('@')?;
    word.chars()
        .all(|c| c.is_alphanumeric() || "._/-:".contains(c))
        .then_some(word)
}

/// What matches `word`, best first: names starting with it, then names holding it, each in the
/// order given (the part's order, then params, then files); case aside.
pub fn matches(all: &[Candidate], word: &str) -> Vec<Candidate> {
    let w = word.to_lowercase();
    let lower: Vec<String> = all.iter().map(|c| c.name.to_lowercase()).collect();
    let starts = all.iter().zip(&lower).filter(|(_, n)| n.starts_with(&w));
    let holds = all
        .iter()
        .zip(&lower)
        .filter(|(_, n)| !n.starts_with(&w) && n.contains(&w));
    starts
        .chain(holds)
        .map(|(c, _)| c.clone())
        .take(MENTION_ROWS)
        .collect()
}

pub const INPUT_ID: &str = "agent-input";
const PAD: f32 = 10.0;
const BAR_H: f32 = 30.0;
const TOOL_H: f32 = 22.0;
const DIFF_ROW: f32 = 15.0;
const ADD_BG: Color32 = Color32::from_rgba_premultiplied(0x1C, 0x4A, 0x2E, 0x90);
const DEL_BG: Color32 = Color32::from_rgba_premultiplied(0x5A, 0x1E, 0x22, 0x90);
const ADD_INK: Color32 = Color32::from_rgb(0x6F, 0xCF, 0x8E);
const DEL_INK: Color32 = Color32::from_rgb(0xF0, 0x80, 0x80);

impl AgentTab {
    /// A backend's word, into the transcript.
    pub fn take(&mut self, e: Event) {
        match e {
            Event::Text(s) => {
                match self.entries.last_mut() {
                    Some(Entry::Agent { text }) if !self.reply_boundary => text.push_str(&s),
                    _ => {
                        let s = s.trim_start_matches('\n').to_string();
                        if !s.is_empty() {
                            self.entries.push(Entry::Agent { text: s });
                        }
                    }
                }
                self.reply_boundary = false;
            }
            Event::ToolStart { id, name } => self.entries.push(Entry::Tool {
                summary: name.clone(),
                id,
                name,
                input: String::new(),
                output: String::new(),
                state: ToolState::Running,
            }),
            Event::ToolInput {
                id,
                summary: s,
                input: i,
            } => {
                if let Some(Entry::Tool { summary, input, .. }) = self.tool(&id) {
                    *summary = s;
                    *input = i;
                }
            }
            Event::ToolDone { id, output: o, error } => {
                if let Some(Entry::Tool { output, state, .. }) = self.tool(&id) {
                    *output = o;
                    *state = if error { ToolState::Failed } else { ToolState::Done };
                }
            }
            Event::Note { text, error } => self.entries.push(Entry::Note { text, error }),
            Event::Done { error, stopped } => {
                self.reply_boundary = true;
                self.running = false;
                for e in &mut self.entries {
                    match e {
                        Entry::Tool { state, .. } if *state == ToolState::Running => {
                            *state = ToolState::Failed
                        }
                        Entry::Permission { state, .. } if *state == Asked::Waiting => {
                            *state = Asked::Dropped
                        }
                        _ => {}
                    }
                }
                if stopped {
                    self.entries.push(Entry::Note {
                        text: "Stopped".into(),
                        error: false,
                    });
                }
                if let Some(text) = error {
                    self.entries.push(Entry::Note { text, error: true });
                }
            }
            Event::Problem { text, fix } => {
                self.running = false;
                self.problem = Some(Problem { text, fix });
            }
            Event::Permission {
                id,
                tool,
                tool_use,
                summary,
                input,
                always,
                ..
            } => self.entries.push(Entry::Permission {
                id,
                tool,
                tool_use,
                summary,
                input,
                always,
                state: Asked::Waiting,
            }),
            Event::Commands(c) => self.commands = c,
            Event::ApiKeyed(k) => self.api_keyed = k,
            Event::Limit {
                reached,
                close,
                resets,
            } => {
                self.limit = (reached || close).then_some((reached, resets));
            }
            Event::Usage {
                secs,
                tokens_in,
                tokens_out,
                cost,
            } => self.usage = Some((secs, tokens_in, tokens_out, cost)),
            Event::Mode(m) => match Mode::from_flag(&m) {
                Some(m) => {
                    self.mode = m;
                    self.mode_other = None;
                }
                None => {
                    if !self.mode_chosen {
                        self.own_mode = Some(m.clone());
                    }
                    self.mode_other = Some(m)
                }
            },
            Event::FileChanged { .. } => {}
        }
    }

    /// The oldest permission still waiting on the person: its id.
    pub fn waiting(&self) -> Option<&str> {
        self.entries.iter().find_map(|e| match e {
            Entry::Permission {
                id,
                state: Asked::Waiting,
                ..
            } => Some(id.as_str()),
            _ => None,
        })
    }

    /// A permission answered: the card says so and the host tells the backend.
    pub fn answer(&mut self, want: &str, a: Answer) {
        for e in &mut self.entries {
            if let Entry::Permission { id, state, .. } = e
                && id == want
                && *state == Asked::Waiting
            {
                *state = match a {
                    Answer::Allow => Asked::Allowed,
                    Answer::Always => Asked::Always,
                    Answer::Deny => Asked::Denied,
                };
                self.out.push(AgentAct::Answer(want.to_string(), a));
                return;
            }
        }
    }

    /// The permission mode, set here: the host passes it on.
    pub fn set_mode(&mut self, m: Mode) {
        if self.mode != m || self.mode_other.is_some() {
            self.mode = m;
            self.mode_other = None;
            self.mode_chosen = true;
            self.out.push(AgentAct::Mode(m));
        }
    }

    /// Back to the person's own settings' mode (`own_mode`), chosen in the mode menu.
    pub fn set_own_mode(&mut self) {
        if let Some(m) = self.own_mode.clone()
            && self.mode_other.as_ref() != Some(&m)
        {
            self.mode_other = Some(m.clone());
            self.mode_chosen = false;
            self.out.push(AgentAct::OwnMode(m));
        }
    }

    /// The person's own settings' mode, in words: the chip's and the menu's.
    pub fn mode_word(flag: &str) -> &str {
        match flag {
            "bypassPermissions" => "Permissions bypassed",
            "dontAsk" => "Don't ask",
            "auto" => "Auto",
            o => o,
        }
    }

    fn tool(&mut self, want: &str) -> Option<&mut Entry> {
        self.entries
            .iter_mut()
            .rev()
            .find(|e| matches!(e, Entry::Tool { id, .. } if id == want))
    }

    /// The input as a message: sent now or queued behind the active turn.
    pub fn send(&mut self) {
        let text = self.input.trim().to_string();
        // the mode, any time: `/mode` opens its menu, `/mode plan` sets it
        if let Some(arg) = text
            .strip_prefix("/mode")
            .filter(|r| r.is_empty() || r.starts_with(' '))
        {
            self.input.clear();
            match arg.trim() {
                "" => self.mode_menu = true,
                "ask" | "default" => self.set_mode(Mode::Ask),
                "edits" | "accept" | "acceptEdits" => self.set_mode(Mode::AcceptEdits),
                "plan" => self.set_mode(Mode::Plan),
                other => self.entries.push(Entry::Note {
                    text: format!("/mode {other}: ask, edits or plan"),
                    error: true,
                }),
            }
            return;
        }
        if !self.running {
            if text == "/clear" || text == "/new" {
                self.input.clear();
                let had = !self.entries.is_empty();
                self.new_chat();
                // said in the empty chat, not kept in it
                self.started_new = had;
                return;
            }
            if text == "/model" {
                self.input.clear();
                self.switcher = true;
                return;
            }
            if text == "/undo" {
                self.input.clear();
                self.undo_last();
                return;
            }
            if text == "/help" {
                self.input.clear();
                // one row a command, whose it is when it isn't the app's
                let said: Vec<String> = command_candidates(&self.commands)
                    .iter()
                    .map(|c| match (c.source, c.what.as_str()) {
                        ("app", w) => format!("/{} · {w}", c.name),
                        (s, "") => format!("/{} · {s}", c.name),
                        (s, w) => format!("/{} · {w} · {s}", c.name),
                    })
                    .collect();
                self.entries.push(Entry::Note {
                    text: said.join("\n"),
                    error: false,
                });
                return;
            }
            if text == "/resume" {
                self.input.clear();
                self.out.push(AgentAct::ListConversations);
                return;
            }
            if let Some(id) = text.strip_prefix("/resume ") {
                self.input.clear();
                self.out.push(AgentAct::ResumeConversation(id.trim().into()));
                return;
            }
        }
        self.markup.commit_text();
        // the view goes as markup does: a picture of it, its thumbnail on the message
        let markup = !self.markup.marks.is_empty() || self.attach_view;
        if (text.is_empty() && !markup && self.pictures.is_empty())
            || self.voice.busy()
            || self.markup.sending
            || self.trust_ask.is_some()
        {
            return;
        }
        self.attach_view = false;
        // what's selected goes with it unless its chip was taken off
        for s in self.suggested_all() {
            if !self.refs.contains(&s) {
                self.refs.push(s);
            }
        }
        let refs = std::mem::take(&mut self.refs);
        let pictures = std::mem::take(&mut self.pictures);
        self.started_new = false;
        // behind a running turn it waits as its queue row only; its card comes when it goes
        // (markup's stays: its picture is taken now, onto the card)
        let shown = !self.running || markup;
        if shown {
            self.entries.push(Entry::User {
                text: text.clone(),
                refs: refs.clone(),
                markup: None,
                pictures: pictures.len(),
            });
        }
        self.input.clear();
        self.problem = None;
        self.usage = None;
        self.recall = None;
        self.running = true;
        self.markup.shown = None;
        self.markup.sending = markup;
        self.out.push(AgentAct::Send {
            text,
            refs,
            markup,
            pictures,
            shown,
        });
    }

    /// A picture pasted or dropped on the tab: it goes with the next message. Not a picture the
    /// models read, or too big: said, and left out.
    pub fn add_picture(&mut self, bytes: Vec<u8>) -> bool {
        let size = bytes.len();
        let refused = if size > crate::agent::PICTURE_BYTES {
            Some(format!(
                "That picture is {:.1} MB · pictures over 5 MB can't be sent: resize it, or send a PNG or JPEG",
                size as f64 / 1048576.0
            ))
        } else {
            match crate::agent::Picture::sniff(bytes) {
                Some(p) => {
                    self.pictures.push(p);
                    None
                }
                None => {
                    Some("Only a PNG, JPEG, GIF or WebP picture can be sent: save it as one of those".into())
                }
            }
        };
        if let Some(text) = refused {
            // guidance, not a failure: said quietly
            self.entries.push(Entry::Note { text, error: false });
            return false;
        }
        true
    }

    fn cancel_dictation(&mut self) {
        if self.voice.busy() {
            self.voice.prefix.clear();
            self.voice.out.push(super::voice::VoiceAct::Cancel);
        }
    }

    pub fn restore_entries(&mut self, entries: Vec<Entry>) {
        self.cancel_dictation();
        self.entries = entries;
        self.started_new = false;
        self.input.clear();
        self.refs.clear();
        self.thumbs.clear();
        self.open.clear();
        self.running = false;
        self.problem = None;
        self.pending = None;
        self.trust_ask = None;
        self.markup = Default::default();
        self.resume_open = false;
        self.resume_error = None;
    }

    pub fn new_chat(&mut self) {
        if self.running {
            return;
        }
        self.cancel_dictation();
        self.entries.clear();
        self.started_new = false;
        self.thumbs.clear();
        self.markup.shown = None;
        self.refs.clear();
        self.running = false;
        self.problem = None;
        self.open.clear();
        self.mode = Mode::Ask;
        self.mode_chosen = false;
        self.mode_other = None;
        self.own_mode = None;
        self.resumed = None;
        self.limit = None;
        self.out.push(AgentAct::NewChat);
    }

    /// The selection, offered for the next message: not already attached, not taken off.
    /// What the offered chip attaches: everything selected, or the one.
    pub fn suggested_all(&self) -> Vec<String> {
        let Some(s) = self.suggested() else { return vec![] };
        if self.selection_all.len() > 1 && self.selection_all.iter().any(|x| x == s) {
            self.selection_all.clone()
        } else {
            vec![s.to_string()]
        }
    }

    pub fn suggested(&self) -> Option<&str> {
        let s = self.selection.as_deref()?;
        (!self.refs.iter().any(|r| r == s) && self.dismissed.as_deref() != Some(s)).then_some(s)
    }

    /// The last turn's changes taken back, each file's card's Undo; said when there's none.
    pub fn undo_last(&mut self) {
        let last = self
            .entries
            .iter()
            .filter_map(|e| match e {
                Entry::Diff {
                    turn, undone: false, ..
                } => Some(*turn),
                _ => None,
            })
            .max();
        let Some(turn) = last else {
            self.entries.push(Entry::Note {
                text: "Nothing to undo: no change of the agent's is in place".into(),
                error: false,
            });
            return;
        };
        let files: Vec<usize> = self
            .entries
            .iter()
            .filter_map(|e| match e {
                Entry::Diff {
                    turn: t,
                    index,
                    undone: false,
                    ..
                } if *t == turn => Some(*index),
                _ => None,
            })
            .collect();
        for index in files {
            self.out.push(AgentAct::Undo(turn, index));
        }
    }

    /// The `@` list's pick, in place of the `@word` typed.
    pub fn take_mention(&mut self) -> bool {
        let Some(m) = self.mention.take() else {
            return false;
        };
        let Some(c) = m.items.get(m.at) else { return false };
        let cut = self.input.len() - m.word.len();
        self.input.truncate(cut);
        self.input.push_str(&c.name);
        self.input.push(' ');
        true
    }

    pub fn attach(&mut self, id: &str) {
        if !id.is_empty() && !self.refs.iter().any(|r| r == id) {
            self.refs.push(id.to_string());
        }
    }

    /// Esc stops a running agent when the tab's showing or its input has the keys.
    pub fn takes_escape(&mut self, ctx: &egui::Context) -> bool {
        if self.voice.busy() {
            ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape));
            self.voice.out.push(super::voice::VoiceAct::Cancel);
            return true;
        }
        // the mode's menu closes first
        if std::mem::take(&mut self.mode_menu) {
            ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape));
            return true;
        }
        // an open `@` list closes first
        if self.mention.as_ref().is_some_and(|m| !m.items.is_empty()) {
            self.mention = None;
            ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape));
            self.mention_off = Some(self.input.clone());
            return true;
        }
        let focused = ctx.memory(|m| m.has_focus(egui::Id::unique(INPUT_ID)));
        if self.tab != SideTab::Agent || !(self.running || focused) {
            return false;
        }
        // a question waiting: Esc says no to it, and the turn goes on
        if let Some(id) = self.waiting().map(str::to_string) {
            ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape));
            self.answer(&id, Answer::Deny);
            return true;
        }
        ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape));
        if self.running {
            self.out.push(AgentAct::Stop);
        } else {
            ctx.memory_mut(|m| m.surrender_focus(egui::Id::unique(INPUT_ID)));
        }
        true
    }
}

/// The message as the agent gets it: the text, then what was attached by id.
pub fn prompt(text: &str, refs: &[String]) -> String {
    if refs.is_empty() {
        return text.to_string();
    }
    let ids: Vec<String> = refs.iter().map(|r| format!("`{r}`")).collect();
    format!("{text}\n\n(Selected in the app: {})", ids.join(", "))
}

/// A tool row's verb in each tense: what it did, what it would do, what it's doing.
const VERBS: [(&str, &str, &str); 27] = [
    ("Applied", "Apply", "Applying"),
    ("Asked", "Ask", "Asking"),
    ("Built", "Build", "Building"),
    ("Compared", "Compare", "Comparing"),
    ("Created", "Create", "Creating"),
    ("Edited", "Edit", "Editing"),
    ("Isolated", "Isolate", "Isolating"),
    ("Kept", "Keep", "Keeping"),
    ("Listed", "List", "Listing"),
    ("Looked", "Look", "Looking"),
    ("Made", "Make", "Making"),
    ("Measured", "Measure", "Measuring"),
    ("Moved", "Move", "Moving"),
    ("Put", "Put", "Putting"),
    ("Queried", "Query", "Querying"),
    ("Ran", "Run", "Running"),
    ("Read", "Read", "Reading"),
    ("Regenerated", "Regenerate", "Regenerating"),
    ("Released", "Release", "Releasing"),
    ("Restored", "Restore", "Restoring"),
    ("Searched", "Search", "Searching"),
    ("Selected", "Select", "Selecting"),
    ("Showed", "Show", "Showing"),
    ("Took", "Take", "Taking"),
    ("Turned", "Turn", "Turning"),
    ("Updated", "Update", "Updating"),
    ("Wrote", "Write", "Writing"),
];

/// The summary with its first word (a past or present verb) in another tense: 0 past, 1
/// present, 2 progressive; as it is when the word isn't a verb the table has.
fn tense(summary: &str, form: usize) -> String {
    let (first, rest) = summary.split_once(' ').unwrap_or((summary, ""));
    let Some(v) = VERBS.iter().find(|v| v.0 == first || v.1 == first) else {
        return summary.to_string();
    };
    let word = [v.0, v.1, v.2][form];
    if rest.is_empty() {
        word.to_string()
    } else {
        format!("{word} {rest}")
    }
}

/// A tool row's words as a question asks them: "Edited plate.fg" (what it did) → "Edit plate.fg"
/// (what it would do).
pub fn present(summary: &str) -> String {
    tense(summary, 1)
}

/// What a running tool is doing: "Querying plate.fg: holes".
pub fn progressive(summary: &str) -> String {
    tense(summary, 2)
}

/// What a declined tool didn't do: "Not edited plate.fg".
pub fn not_done(summary: &str) -> String {
    let past = tense(summary, 0);
    // the participle where it isn't the past
    let past = match past.split_once(' ') {
        Some((v, rest)) => {
            let p = match v {
                "Ran" => "Run",
                "Took" => "Taken",
                "Showed" => "Shown",
                "Wrote" => "Written",
                v => v,
            };
            format!("{p}: {rest}")
        }
        None => past,
    };
    let mut c = past.chars();
    match c.next() {
        Some(f) => format!("Not {}{}", f.to_lowercase(), c.as_str()),
        None => "Not run".into(),
    }
}

/// "Agent: " and the request's first line, as its undo step reads.
pub fn undo_label(text: &str) -> String {
    let first = text.lines().find(|l| !l.trim().is_empty()).unwrap_or("").trim();
    let mut s: String = first.chars().take(48).collect();
    if first.chars().count() > 48 {
        s.push('…');
    }
    format!("Agent: {s}")
}

/// A line-by-line diff of two texts, the unchanged runs cut to `context` lines either side.
pub fn diff(old: &str, new: &str, context: usize) -> Vec<Line> {
    let a: Vec<&str> = old.lines().collect();
    let b: Vec<&str> = new.lines().collect();
    let pre = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    let suf = a[pre..]
        .iter()
        .rev()
        .zip(b[pre..].iter().rev())
        .take_while(|(x, y)| x == y)
        .count();
    let (am, bm) = (&a[pre..a.len() - suf], &b[pre..b.len() - suf]);
    let mut mid: Vec<Line> = Vec::new();
    if am.len() * bm.len() > 4_000_000 {
        mid.extend(am.iter().map(|l| Line::Del(l.to_string())));
        mid.extend(bm.iter().map(|l| Line::Add(l.to_string())));
    } else {
        // longest common subsequence, back to front
        let (n, m) = (am.len(), bm.len());
        let mut t = vec![0u32; (n + 1) * (m + 1)];
        for i in (0..n).rev() {
            for j in (0..m).rev() {
                t[i * (m + 1) + j] = if am[i] == bm[j] {
                    t[(i + 1) * (m + 1) + j + 1] + 1
                } else {
                    t[(i + 1) * (m + 1) + j].max(t[i * (m + 1) + j + 1])
                };
            }
        }
        let (mut i, mut j) = (0, 0);
        while i < n || j < m {
            if i < n && j < m && am[i] == bm[j] {
                mid.push(Line::Same(am[i].to_string()));
                i += 1;
                j += 1;
            } else if i < n && (j == m || t[(i + 1) * (m + 1) + j] >= t[i * (m + 1) + j + 1]) {
                mid.push(Line::Del(am[i].to_string()));
                i += 1;
            } else {
                mid.push(Line::Add(bm[j].to_string()));
                j += 1;
            }
        }
    }
    let mut all: Vec<Line> = a[..pre].iter().map(|l| Line::Same(l.to_string())).collect();
    all.extend(mid);
    all.extend(a[a.len() - suf..].iter().map(|l| Line::Same(l.to_string())));
    // keep `context` unchanged lines around each change
    let changed: Vec<bool> = all.iter().map(|l| !matches!(l, Line::Same(_))).collect();
    let near = |k: usize| {
        let lo = k.saturating_sub(context);
        let hi = (k + context + 1).min(all.len());
        changed[lo..hi].iter().any(|c| *c)
    };
    let mut out = Vec::new();
    let mut skipped = 0;
    for (k, l) in all.iter().enumerate() {
        if near(k) {
            if skipped > 0 {
                out.push(Line::Skip(skipped));
                skipped = 0;
            }
            out.push(l.clone());
        } else {
            skipped += 1;
        }
    }
    if skipped > 0 && !out.is_empty() {
        out.push(Line::Skip(skipped));
    }
    out
}

pub fn counts(lines: &[Line]) -> (usize, usize) {
    let add = lines.iter().filter(|l| matches!(l, Line::Add(_))).count();
    let del = lines.iter().filter(|l| matches!(l, Line::Del(_))).count();
    (add, del)
}

fn base(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

fn s<'a>(j: &'a Json, k: &str) -> &'a str {
    j.get(k).and_then(Json::str).unwrap_or("")
}

fn one_line(s: &str, n: usize) -> String {
    let l = s.lines().next().unwrap_or("").trim();
    let mut o: String = l.chars().take(n).collect();
    if l.chars().count() > n || s.lines().nth(1).is_some() {
        o.push('…');
    }
    o
}

/// A command less its leading `cd DIR &&`.
fn strip_cd(c: &str) -> &str {
    let t = c.trim_start();
    match t.strip_prefix("cd ").and_then(|r| r.split_once("&&")) {
        Some((_, rest)) => rest.trim_start(),
        None => t,
    }
}

/// A tool call as its row reads, and what it was given in full.
pub fn tool_summary(name: &str, input: &Json) -> (String, String) {
    let name = name
        .strip_prefix("mcp__forgecad__")
        .or_else(|| name.strip_prefix("app_"))
        .unwrap_or(name);
    let list = |k: &str| match input.get(k) {
        Some(Json::Arr(a)) => a.iter().filter_map(Json::str).collect::<Vec<_>>().join(", "),
        _ => String::new(),
    };
    let on = |k: &str| input.get(k) == Some(&Json::Bool(true));
    let whole = || {
        let mut t = String::new();
        input.write(&mut t);
        t
    };
    match name {
        "Read" | "read_file" => {
            let f = s(input, "file_path");
            (
                format!("Read {}", if f.is_empty() { "the file" } else { base(f) }),
                whole(),
            )
        }
        "Edit" | "MultiEdit" | "replace_text" => {
            let f = s(input, "file_path");
            let (old, new) = (s(input, "old_string"), s(input, "new_string"));
            let (old, new) = if name == "replace_text" {
                (s(input, "old"), s(input, "new"))
            } else {
                (old, new)
            };
            let mut d = String::new();
            for l in old.lines() {
                d.push_str(&format!("- {l}\n"));
            }
            for l in new.lines() {
                d.push_str(&format!("+ {l}\n"));
            }
            (
                format!("Edited {}", if f.is_empty() { "the file" } else { base(f) }),
                d,
            )
        }
        "Write" => (format!("Wrote {}", base(s(input, "file_path"))), whole()),
        "Bash" => {
            let c = s(input, "command");
            let what = match s(input, "description") {
                "" => format!("Ran {}", one_line(strip_cd(c), 60)),
                d => one_line(d, 60),
            };
            (what, c.to_string())
        }
        "Grep" => (
            format!("Searched for {}", one_line(s(input, "pattern"), 40)),
            whole(),
        ),
        "Glob" => (format!("Listed {}", one_line(s(input, "pattern"), 40)), whole()),
        "apply_op" => {
            let mut t = String::new();
            if let Some(op) = input.get("op") {
                op.write(&mut t);
            }
            (format!("Applied {}", one_line(&t, 50)), t)
        }
        "run_check" => ("Built and checked the part".into(), String::new()),
        "create_file" => (format!("Made {}", s(input, "name")), s(input, "text").to_string()),
        "TodoWrite" => ("Updated its plan".into(), whole()),
        "tree" => ("Looked at the browser".into(), whole()),
        "set_visibility" => (
            format!(
                "{} {}",
                if on("visible") { "Showed" } else { "Hid" },
                one_line(&list("ids"), 50)
            ),
            whole(),
        ),
        "isolate" => (format!("Isolated {}", s(input, "id")), whole()),
        "show_all" => ("Showed everything".into(), whole()),
        "select" => (format!("Selected {}", s(input, "id")), whole()),
        "set_view" => (format!("Turned the view to {}", s(input, "view")), whole()),
        "drive_joint" => (
            format!(
                "Moved {} to {} for a look",
                s(input, "joint"),
                input.get("value").map(|v| v.to_string()).unwrap_or_default()
            ),
            whole(),
        ),
        "return_to_design" => ("Put the joints back".into(), whole()),
        "keep_position" => ("Kept the position in the file".into(), whole()),
        "screenshot" => ("Looked at the view".into(), whole()),
        "measure" => (
            match s(input, "b") {
                "" => format!("Measured {}", s(input, "a")),
                b => format!("Measured {} to {b}", s(input, "a")),
            },
            whole(),
        ),
        "undo_view" => ("Took back its last view step".into(), whole()),
        "fg_run" | "fg_check" => (format!("Built and checked {}", s(input, "file")), whole()),
        "fg_edit" => (format!("Applied an edit to {}", s(input, "file")), whole()),
        "fg_handles" => (format!("Listed the parameters of {}", s(input, "file")), whole()),
        "project_info" => ("Listed the project files".into(), whole()),
        "project_enable_history" => ("Turned on project history".into(), whole()),
        "project_mkdir" => (format!("Created {}", s(input, "path")), whole()),
        "project_read" => (format!("Read {}", s(input, "file")), whole()),
        "project_write" => (format!("Wrote {}", s(input, "file")), whole()),
        "fg_history" => (format!("Read the history of {}", s(input, "file")), whole()),
        "fg_versions" => (format!("Listed versions of {}", s(input, "file")), whole()),
        "fg_create_version" => (format!("Created version {}", s(input, "name")), whole()),
        "fg_compare" => (
            format!("Compared {} to {}", s(input, "base"), s(input, "target")),
            whole(),
        ),
        "fg_restore" => (
            format!("Restored {} to {}", s(input, "file"), s(input, "to")),
            whole(),
        ),
        "fg_releases" => (format!("Listed releases of {}", s(input, "file")), whole()),
        "fg_release" => (format!("Released {}", s(input, "file")), whole()),
        "fg_regenerate_release" => (format!("Regenerated release {}", s(input, "revision")), whole()),
        "fg_reference" => ("Read the Forge reference".into(), whole()),
        "fg_query" | "query" => (format!("Asked {}", s(input, "query")), whole()),
        _ => (name.to_string(), whole()),
    }
}

/// The reply's text laid out: `code` and fenced blocks in mono, ** dropped.
fn rich(text: &str, width: f32, ink: Color32, code_ink: Color32) -> LayoutJob {
    let mut job = LayoutJob::default();
    job.wrap.max_width = width;
    let fmt = |font: FontId, color: Color32| TextFormat {
        font_id: font,
        color,
        ..Default::default()
    };
    let plain = fmt(sans(13.0), ink);
    let code = fmt(mono(12.0), code_ink);
    let mut fence = false;
    for (k, line) in text.split('\n').enumerate() {
        if k > 0 {
            job.append("\n", 0.0, plain.clone());
        }
        if line.trim_start().starts_with("```") {
            fence = !fence;
            continue;
        }
        if fence {
            job.append(line, 0.0, code.clone());
            continue;
        }
        for (part, f) in md_line(line, ink, code_ink) {
            job.append(&part, 0.0, f);
        }
    }
    job
}

/// One line of Markdown as runs of text and their look: a heading in bold, a bullet as •, a
/// quote dimmed, a table's row in mono (its columns line up), `code`, **bold**.
fn md_line(line: &str, ink: Color32, code_ink: Color32) -> Vec<(String, TextFormat)> {
    let fmt = |font: FontId, color: Color32| TextFormat {
        font_id: font,
        color,
        ..Default::default()
    };
    let t = line.trim_start();
    let indent = &line[..line.len() - t.len()];
    // a table's row, or its rule: as it is, in mono
    if t.starts_with('|') {
        // its rule (|---|:--|) faint: the rows are what's read
        let rule = t.chars().all(|c| "|-: ".contains(c));
        return vec![(
            line.to_string(),
            fmt(mono(12.0), if rule { ink.gamma_multiply(0.35) } else { ink }),
        )];
    }
    if let Some(h) = ["### ", "## ", "# "].iter().find_map(|p| t.strip_prefix(p)) {
        return inline(h, fmt(semi(14.0), ink), code_ink);
    }
    if let Some(q) = t.strip_prefix("> ").or_else(|| (t == ">").then_some("")) {
        return inline(q, fmt(sans(13.0), ink.gamma_multiply(0.7)), code_ink);
    }
    if t == "---" || t == "***" {
        return vec![("────────".into(), fmt(sans(13.0), ink.gamma_multiply(0.4)))];
    }
    let mut out = vec![];
    let body = match t.strip_prefix("- ").or_else(|| t.strip_prefix("* ")) {
        Some(b) => {
            out.push((format!("{indent}• "), fmt(sans(13.0), ink)));
            b
        }
        None => {
            out.push((indent.to_string(), fmt(sans(13.0), ink)));
            t
        }
    };
    out.extend(inline(body, fmt(sans(13.0), ink), code_ink));
    out
}

/// A line's `code` and **bold** runs, the rest in `base`.
fn inline(text: &str, base: TextFormat, code_ink: Color32) -> Vec<(String, TextFormat)> {
    let mut out = vec![];
    for (n, part) in text.split('`').enumerate() {
        if part.is_empty() {
            continue;
        }
        if n % 2 == 1 {
            out.push((
                part.to_string(),
                TextFormat {
                    font_id: mono(12.0),
                    color: code_ink,
                    ..Default::default()
                },
            ));
            continue;
        }
        for (m, run) in part.split("**").enumerate() {
            if run.is_empty() {
                continue;
            }
            let mut f = base.clone();
            if m % 2 == 1 {
                f.font_id = semi(f.font_id.size);
            }
            out.push((run.to_string(), f));
        }
    }
    out
}

/// A reply cut into prose and fenced code blocks, in order: `(is code, text)`.
fn segments(text: &str) -> Vec<(bool, &str)> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(open) = rest.find("```") {
        let before = rest[..open].trim_end_matches('\n');
        if !before.trim().is_empty() {
            out.push((false, before));
        }
        // the fence's own line (its language) is not code
        let body = &rest[open + 3..];
        let body = body.find('\n').map_or("", |n| &body[n + 1..]);
        match body.find("```") {
            Some(close) => {
                out.push((true, body[..close].trim_end_matches('\n')));
                let after = &body[close + 3..];
                rest = after.find('\n').map_or("", |n| &after[n + 1..]);
            }
            // still streaming in: the code so far
            None => {
                out.push((true, body.trim_end_matches('\n')));
                rest = "";
            }
        }
    }
    if !rest.trim().is_empty() {
        out.push((false, rest.trim_start_matches('\n')));
    }
    out
}

/// Prose cut into its Markdown tables (runs of `|` lines) and the rest: (is a table, the run).
fn tables(text: &str) -> Vec<(bool, &str)> {
    let mut out: Vec<(bool, &str)> = Vec::new();
    let mut start = 0;
    let mut kind = None;
    let mut at = 0;
    for line in text.split_inclusive('\n') {
        let is = line.trim_start().starts_with('|');
        if kind.is_some_and(|k| k != is) {
            let run = text[start..at].trim_matches('\n');
            if !run.trim().is_empty() {
                out.push((kind.unwrap(), run));
            }
            start = at;
        }
        kind = Some(is);
        at += line.len();
    }
    if let Some(k) = kind {
        let run = text[start..].trim_matches('\n');
        if !run.trim().is_empty() {
            out.push((k, run));
        }
    }
    out
}

/// A Markdown table's cells, row by row, its rule (`|---|:--|`) left out.
fn table_cells(run: &str) -> Vec<Vec<&str>> {
    run.lines()
        .map(str::trim)
        .filter(|l| !l.chars().all(|c| "|-: ".contains(c)))
        .map(|l| {
            let l = l.strip_prefix('|').unwrap_or(l);
            let l = l.strip_suffix('|').unwrap_or(l);
            l.split('|').map(str::trim).collect()
        })
        .collect()
}

/// A Markdown table as a grid: its header in the bold face, rows striped, sideways scroll when
/// it's wider than the panel.
fn table_grid(
    ui: &mut Ui,
    t: &Theme,
    run: &str,
    inner: f32,
    code_ink: Color32,
    id: impl std::hash::Hash + std::fmt::Debug + Copy,
) {
    let rows = table_cells(run);
    let cols = rows.iter().map(Vec::len).max().unwrap_or(1).max(1) as f32;
    // the first column (ids, names) as wide as it is, up to 40% of the panel; the rest share
    // what's left
    let first = rows
        .iter()
        .enumerate()
        .filter_map(|(n, r)| {
            let font = if n == 0 { semi(12.0) } else { sans(12.0) };
            // as it's drawn: `code` in mono is wider than the plain face
            r.first().map(|c| {
                let base = TextFormat {
                    font_id: font,
                    color: t.ink,
                    ..Default::default()
                };
                let mut job = LayoutJob::default();
                for (s, f) in inline(c, base, code_ink) {
                    job.append(&s, 0.0, f);
                }
                ui.fonts_mut(|f| f.layout_job(job).size().x)
            })
        })
        .fold(0.0, f32::max)
        .min(inner * 0.4)
        .max(40.0);
    let rest_w = if cols > 1.0 {
        ((inner - first - 14.0 * (cols - 1.0)) / (cols - 1.0)).max(48.0)
    } else {
        first
    };
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.add_space(PAD);
        egui::ScrollArea::horizontal()
            .id_salt(id)
            .max_width(inner)
            .auto_shrink([true, true])
            .show(ui, |ui| {
                egui::Grid::new(id)
                    .striped(true)
                    .spacing(vec2(14.0, 4.0))
                    .show(ui, |ui| {
                        for (n, row) in rows.iter().enumerate() {
                            let font = if n == 0 { semi(12.0) } else { sans(12.0) };
                            for (c, cell) in row.iter().enumerate() {
                                let col_w = if c == 0 { first + 1.0 } else { rest_w };
                                let mut job = LayoutJob::default();
                                let base = TextFormat {
                                    font_id: font.clone(),
                                    color: t.ink,
                                    ..Default::default()
                                };
                                for (s, f) in inline(cell, base, code_ink) {
                                    job.append(&s, 0.0, f);
                                }
                                // each column its share of the panel: a longer cell ends in …,
                                // whole on hover
                                ui.scope(|ui| {
                                    ui.set_max_width(col_w);
                                    ui.add(egui::Label::new(job).selectable(true).truncate());
                                });
                            }
                            ui.end_row();
                        }
                    });
            });
    });
    ui.add_space(4.0);
}

/// Laid-out text put where it goes, selectable: a drag across messages, a double click on a
/// word, Ctrl+C.
fn selectable(ui: &mut Ui, at: egui::Pos2, g: std::sync::Arc<egui::Galley>) {
    let r = Rect::from_min_size(at, g.size());
    ui.put(r, egui::Label::new(egui::WidgetText::Galley(g)).selectable(true));
}

/// The same wrapping and line spacing for composer measurement and editing.
fn input_galley(ui: &Ui, text: &str, width: f32) -> std::sync::Arc<egui::Galley> {
    let row_height = ui.fonts_mut(|f| f.row_height(&sans(13.0))) + ui.spacing().extra_text_line_spacing;
    let mut job = plain_job(text, sans(13.0), ui.visuals().text_color(), width);
    job.keep_trailing_whitespace = true;
    for section in &mut job.sections {
        section.format.line_height = Some(row_height);
    }
    ui.fonts_mut(|f| f.layout_job(job))
}

fn plain_job(text: &str, font: FontId, ink: Color32, width: f32) -> LayoutJob {
    let mut job = LayoutJob::single_section(
        text.to_string(),
        TextFormat {
            font_id: font,
            color: ink,
            ..Default::default()
        },
    );
    job.wrap.max_width = width;
    job
}

impl Shell {
    /// The panel's tabs, Model and Agent, with the part's name at the right on Model's.
    pub(super) fn side_tabs(&mut self, ui: &mut Ui, y: f32, w: f32, right: &str) {
        let t = self.theme;
        let p = ui.painter().clone();
        let h = Rect::from_min_size(pos2(0.0, y), vec2(w, HEAD_H));
        p.rect_filled(h, 0.0, t.raised);
        p.hline(0.0..=w, y + HEAD_H - 0.5, Stroke::new(1.0, t.line));
        let mut x = 4.0;
        for (tab, word) in [
            (SideTab::Model, "Model"),
            (SideTab::Agent, "Agent"),
            (SideTab::Versions, "Versions"),
        ] {
            let on = self.agent.tab == tab;
            let tw = ui
                .fonts_mut(|f| f.layout_no_wrap(word.to_string(), semi(12.0), t.ink))
                .size()
                .x
                + 22.0;
            let r = Rect::from_min_size(pos2(x, y + 3.0), vec2(tw, HEAD_H - 3.0));
            let resp = ui.interact(r, ui.scope_id().with(("side-tab", tab)), Sense::click());
            resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::Tab, true, on, word));
            if resp.clicked() {
                if tab == SideTab::Versions && self.agent.tab != tab {
                    self.versions.out.push(super::versions::VersionsAct::Load);
                }
                self.agent.tab = tab;
            }
            if on {
                p.rect_filled(r, 3.0, t.surface);
                p.rect_filled(
                    Rect::from_min_size(pos2(r.left(), r.bottom() - 2.0), vec2(tw, 2.0)),
                    0.0,
                    t.accent,
                );
            } else if resp.hovered() {
                p.rect_filled(r, 3.0, t.hover);
            }
            let ink = if on || resp.hovered() { t.ink } else { t.ink2 };
            p.text(
                r.center(),
                Align2::CENTER_CENTER,
                word,
                if on { semi(12.0) } else { sans(12.0) },
                ink,
            );
            // a running agent shows on its tab from the Model tab
            // a running agent shows on its tab from the Model tab; one waiting on you, louder
            if tab == SideTab::Agent && !on && self.agent.waiting().is_some() {
                p.circle_filled(pos2(r.right() - 6.0, r.top() + 7.0), 4.5, t.warn);
                resp.clone()
                    .on_hover_text("The agent is waiting for your permission");
            } else if tab == SideTab::Agent && !on && self.agent.running {
                p.circle_filled(pos2(r.right() - 6.0, r.top() + 7.0), 3.0, t.focus);
            }
            self.note(format!("side:tab:{word}"), r);
            x += tw + 2.0;
        }
        if self.agent.tab == SideTab::Model {
            let room = w - x - 16.0;
            // the name and its released revision; where both don't fit, the revision alone
            let rev = right.split_once(" · Rev ").map(|(_, r)| format!("Rev {r}"));
            let fits = [
                Some(right.to_string()),
                rev,
                Some(right.split(" · Rev ").next().unwrap_or("").to_string()),
            ]
            .into_iter()
            .flatten()
            .map(|s| ui.fonts_mut(|f| f.layout_no_wrap(s, sans(12.0), t.ink2)))
            .find(|g| g.size().x <= room);
            if let Some(g) = fits {
                p.galley(
                    pos2(w - 10.0 - g.size().x, y + (HEAD_H - g.size().y) * 0.5),
                    g,
                    t.ink2,
                );
            }
        } else if self.agent.tab == SideTab::Agent {
            let r = Rect::from_min_size(pos2(w - 30.0, y + 3.0), vec2(24.0, 24.0));
            let resp = ui
                .interact(r, ui.scope_id().with("agent-new"), Sense::click())
                .on_hover_text("New chat");
            resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, "New chat"));
            if resp.hovered() {
                p.rect_filled(r, 4.0, t.hover);
            }
            let (c, ink) = (r.center(), if resp.hovered() { t.ink } else { t.ink2 });
            p.hline(c.x - 6.0..=c.x + 6.0, c.y, Stroke::new(1.5, ink));
            p.vline(c.x, c.y - 6.0..=c.y + 6.0, Stroke::new(1.5, ink));
            self.note("agent:new", r);
            if resp.clicked() {
                self.agent.new_chat();
            }
        }
    }

    /// The selection as the agent names it: the face picked in the view, a browser row, an
    /// assembly's occurrence.
    pub fn agent_selection(&self) -> Option<String> {
        if let Some(d) = &self.drawing {
            return d.selected.clone();
        }
        let fg = self.fg.as_ref()?;
        if let Some(a) = &fg.asm {
            return a.selected.clone();
        }
        match &fg.picked {
            Some(Ok(name)) => Some(name.clone()),
            _ => fg.selected.clone(),
        }
    }

    /// A browser row dragged onto the Agent tab: attached to the next message.
    pub(super) fn agent_drag(&mut self, ui: &mut Ui) {
        let (origin, at, down, released) = ui.input(|i| {
            (
                i.pointer.press_origin(),
                i.pointer.latest_pos(),
                i.pointer.primary_down(),
                i.pointer.primary_released(),
            )
        });
        let (Some(o), Some(at)) = (origin, at) else { return };
        if o.distance(at) < 8.0 {
            return;
        }
        let row = self.places.iter().find_map(|(n, r)| {
            let key = n.strip_prefix("fg:row:").or_else(|| n.strip_prefix("asm:row:"))?;
            r.contains(o).then(|| key.to_string())
        });
        let Some(key) = row else { return };
        let Some(tab) = self.place("side:tab:Agent") else {
            return;
        };
        let t = self.theme;
        if down {
            let p = ui.ctx().layer_painter(egui::LayerId::new(
                egui::Order::Tooltip,
                egui::Id::unique("agent-drag"),
            ));
            let g = ui.fonts_mut(|f| f.layout_no_wrap(format!("@{key}"), mono(12.0), t.ink));
            let r = Rect::from_min_size(at + vec2(12.0, 8.0), g.size() + vec2(12.0, 6.0));
            p.rect_filled(r, 4.0, t.raised);
            p.rect_stroke(r, 4.0, Stroke::new(1.0, t.focus), StrokeKind::Inside);
            p.galley(r.min + vec2(6.0, 3.0), g, t.ink);
            let hot = tab.contains(at);
            p.rect_stroke(
                tab,
                3.0,
                Stroke::new(if hot { 2.0 } else { 1.0 }, t.focus),
                StrokeKind::Inside,
            );
        } else if released && tab.contains(at) {
            self.agent.attach(&key);
            self.agent.tab = SideTab::Agent;
        }
    }

    /// The Agent tab: the backend's switcher, what it's working on, the chat, the input.
    pub(super) fn agent_ui(&mut self, ui: &mut Ui, area: Rect) {
        // the chip the pointer is on, found again each frame
        self.agent.hover_ref = None;
        let t = self.theme;
        // in a page the agent is the person's own Claude Code, on their machine behind
        // `forgecad web --agent` (agent/bridge.rs); with no bridge, how to start one
        #[cfg(target_arch = "wasm32")]
        let bridged = crate::agent::bridge::ready();
        #[cfg(not(target_arch = "wasm32"))]
        let bridged = true;
        if !bridged {
            let mut places = std::mem::take(&mut self.places);
            ui.scope_builder(
                egui::UiBuilder::new().max_rect(area.shrink2(vec2(PAD, 12.0))),
                |ui| no_bridge(ui, &t, &mut places),
            );
            self.places = places;
            return;
        }
        self.agent.selection = self.agent_selection();
        self.agent.selection_all = match (&self.drawing, self.fg.as_ref()) {
            (None, Some(fg)) if fg.asm.is_none() => fg.picked_all.clone(),
            _ => Vec::new(),
        };
        let drawing = self.drawing.as_ref().map(|d| d.name());
        self.agent.file = drawing
            .or_else(|| self.fg.as_ref().map(|f| base(&f.file).to_string()))
            .filter(|f| !f.is_empty());
        let p = ui.painter().clone();
        let w = area.width();

        // the backend and model, a click to switch
        let bar = Rect::from_min_size(area.min, vec2(w, BAR_H));
        let label = self.agent.settings.label();
        let g = ui.fonts_mut(|f| f.layout_no_wrap(label.clone(), medium(12.0), t.ink));
        let bw = (g.size().x + 30.0).min((w - 2.0 * PAD - 58.0).max(40.0));
        let br = Rect::from_min_size(pos2(bar.left() + PAD - 4.0, bar.top() + 4.0), vec2(bw, 22.0));
        let resp = ui.interact(br, ui.scope_id().with("agent-switch"), Sense::click());
        resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &label));
        if resp.hovered() || self.agent.switcher {
            p.rect_filled(br, 4.0, t.hover);
        }
        let clip = p.with_clip_rect(br.shrink2(vec2(2.0, 0.0)));
        clip.galley(pos2(br.left() + 6.0, br.center().y - g.size().y * 0.5), g, t.ink);
        icons::draw(
            &p,
            pos2(br.right() - 18.0, br.top() + 4.0),
            14.0,
            "chevron",
            1.4,
            t.ink2,
        );
        self.note("agent:switch", br);
        if resp.clicked() {
            self.agent.switcher = !self.agent.switcher;
        }
        if self.agent.switcher {
            self.agent_switcher(ui.ctx(), br);
        }

        let chats = Rect::from_min_size(pos2(bar.right() - PAD - 48.0, bar.top() + 4.0), vec2(48.0, 22.0));
        let response = ui
            .interact(chats, ui.scope_id().with("agent-resume"), Sense::click())
            .on_hover_text("Resume a saved conversation (/resume)");
        response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, !self.agent.running, "Chats"));
        if response.hovered() {
            p.rect_filled(chats, 4.0, t.hover);
        }
        p.text(
            chats.center(),
            Align2::CENTER_CENTER,
            "Chats",
            sans(12.0),
            if self.agent.running { t.ink2 } else { t.ink },
        );
        self.note("agent:resume", chats);
        if response.clicked() && !self.agent.running {
            self.agent.out.push(AgentAct::ListConversations);
        }

        // what may run without asking (Claude Code's permission mode); click or Shift+Tab to change
        if self.agent.settings.kind == agent_settings::Kind::ClaudeCode {
            let m = self.agent.mode;
            let other = self.agent.mode_other.clone();
            // a mode from the person's own settings the tab doesn't set, in words
            let full = other.as_deref().map_or(m.word(), AgentTab::mode_word);
            // narrow: a bypass still says so, in a word
            let room = chats.left() - 6.0 - (br.right() + 4.0);
            let mut word = full;
            let mut g = ui.fonts_mut(|f| f.layout_no_wrap(word.to_string(), sans(11.0), t.ink));
            if g.size().x + 14.0 > room && other.as_deref() == Some("bypassPermissions") {
                word = "Bypassed";
                g = ui.fonts_mut(|f| f.layout_no_wrap(word.to_string(), sans(11.0), t.ink));
            }
            let mw = g.size().x + 14.0;
            let mr = Rect::from_min_size(pos2(chats.left() - 6.0 - mw, bar.top() + 6.0), vec2(mw, 18.0));
            // only where it doesn't cover the backend's name
            if mr.left() > br.right() + 4.0 {
                let tip = match other.as_deref() {
                    Some("bypassPermissions") => "Your Claude settings bypass permissions: the agent runs every tool without asking. Click to choose another mode for this chat".to_string(),
                    Some(o) => format!("Your Claude Code settings' mode, {o} (click to choose another)"),
                    None => m.tip().to_string(),
                };
                let resp = ui
                    .interact(mr, ui.scope_id().with("agent-mode"), Sense::click())
                    .on_hover_text(tip);
                resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, full));
                let (fill, ink) = match m {
                    _ if other.is_some() => (t.warn.gamma_multiply(0.25), t.ink),
                    Mode::Ask => (t.field, t.ink2),
                    Mode::AcceptEdits => (t.success.gamma_multiply(0.25), t.ink),
                    Mode::Plan => (t.focus.gamma_multiply(0.25), t.ink),
                };
                p.rect_filled(mr, 9.0, if resp.hovered() { t.hover } else { fill });
                p.galley(pos2(mr.left() + 7.0, mr.center().y - g.size().y * 0.5), g, ink);
                self.note("agent:mode", mr);
                // a click opens the three, each said: a misclick can't switch it unseen
                if resp.clicked() {
                    self.agent.mode_menu = !self.agent.mode_menu;
                }
                if self.agent.mode_menu {
                    self.mode_menu(ui.ctx(), mr);
                }
            }
        }
        // a question waiting, on screen, and nothing with the keys: Enter allows it
        if let Some(id) = self.agent.waiting().map(str::to_string)
            && self.agent.card_visible
            && ui.memory(|m| m.focused().is_none())
            && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter))
        {
            self.agent.answer(&id, Answer::Allow);
        }
        self.agent_resume_picker(ui.ctx());

        // what it's working on
        let ctx_y = bar.bottom();
        let on = match &self.agent.file {
            Some(f) => f.clone(),
            None => self.agent.project.clone().unwrap_or_else(|| "project".into()),
        };
        p.text(
            pos2(PAD, ctx_y + 9.0),
            Align2::LEFT_CENTER,
            "On",
            sans(11.0),
            t.ink2,
        );
        let og = ui.fonts_mut(|f| f.layout_no_wrap(on, mono(11.0), t.ink));
        p.with_clip_rect(Rect::from_min_max(
            pos2(PAD + 18.0, ctx_y),
            pos2(w - PAD, ctx_y + 18.0),
        ))
        .galley(pos2(PAD + 20.0, ctx_y + 9.0 - og.size().y * 0.5), og, t.ink);
        let top = ctx_y + 20.0;
        p.hline(0.0..=w, top - 0.5, Stroke::new(1.0, t.line));

        // Measure wrapped rows at the editor's width, including pasted paragraphs.
        let chips = if self.agent.refs.is_empty()
            && self.agent.suggested().is_none()
            && self.agent.pictures.is_empty()
        {
            0.0
        } else {
            26.0
        };
        let text_width = (w - 2.0 * (PAD - 2.0) - 8.0).max(24.0);
        let row_height = ui.fonts_mut(|f| f.row_height(&sans(13.0))) + ui.spacing().extra_text_line_spacing;
        let available = (area.bottom() - top).max(0.0);
        let max_input = (8.0 * row_height + 44.0 + chips)
            .min((available * 0.6).max(78.0 + chips))
            .min(available);
        let text_height = input_galley(ui, &self.agent.input, text_width).size().y;
        let input_h = (text_height + 44.0 + chips).max(78.0 + chips).min(max_input);
        let input = Rect::from_min_max(pos2(area.left(), area.bottom() - input_h), area.max);
        let chat = Rect::from_min_max(pos2(area.left(), top), pos2(area.right(), input.top()));
        self.agent_chat(ui, chat);
        self.agent_input(ui, input);
        // the running turn's start, for its time so far
        if !self.agent.running {
            self.agent.turn_started = None;
        } else if self.agent.turn_started.is_none() {
            self.agent.turn_started = Some(ui.input(|i| i.time));
        }
        if self.agent.running {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(120));
        }
    }

    fn agent_resume_picker(&mut self, ctx: &egui::Context) {
        if !self.agent.resume_open {
            return;
        }
        let mut open = true;
        let mut selected = None;
        let mut new = false;
        let mut places = Vec::new();
        let theme = self.theme;
        egui::Window::new("Resume conversation")
            .id(egui::Id::unique("agent-resume-window"))
            .open(&mut open)
            .default_size(vec2(480.0, 360.0))
            .show(ctx, |ui| {
                ui.label(self.agent.project.as_deref().unwrap_or("Current project"));
                let search = ui.add(
                    egui::TextEdit::singleline(&mut self.agent.resume_filter)
                        .hint_text("Search conversations")
                        .desired_width(f32::INFINITY),
                );
                places.push(("agent:resume:search".into(), search.rect));
                if let Some(error) = &self.agent.resume_error {
                    ui.colored_label(theme.error, error);
                }
                ui.separator();
                egui::ScrollArea::vertical()
                    .id_salt("saved-chats")
                    .max_height(260.0)
                    .show(ui, |ui| {
                        if self.agent.saved_chats.is_empty() {
                            ui.label("No saved conversations for this project yet.");
                        }
                        let filter = self.agent.resume_filter.to_lowercase();
                        for chat in &self.agent.saved_chats {
                            if !format!("{} {}", chat.title, chat.backend)
                                .to_lowercase()
                                .contains(&filter)
                            {
                                continue;
                            }
                            let age = ago(chat.updated);
                            let button = ui.add_sized(
                                vec2(ui.available_width(), 48.0),
                                egui::Button::new(format!("{}\n{} · {}", chat.title, chat.backend, age)),
                            );
                            places.push((format!("agent:resume:{}", chat.id), button.rect));
                            if button.clicked() {
                                selected = Some(chat.id.clone());
                            }
                        }
                    });
                ui.separator();
                let button = ui.button("New conversation");
                places.push(("agent:resume:new".into(), button.rect));
                if button.clicked() {
                    new = true;
                }
            });
        self.agent.resume_open = open;
        self.places.extend(places);
        if let Some(id) = selected {
            self.agent.out.push(AgentAct::ResumeConversation(id));
        }
        if new {
            self.agent.resume_open = false;
            self.agent.new_chat();
        }
    }

    fn agent_switcher(&mut self, ctx: &egui::Context, under: Rect) {
        let t = self.theme;
        let mut close = false;
        let mut open_settings = false;
        let mut rows: Vec<(&str, Rect)> = Vec::new();
        let area = egui::Area::new(egui::Id::unique("agent-switcher"))
            .order(egui::Order::Foreground)
            .fixed_pos(pos2(under.left(), under.bottom() + 2.0))
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(t.raised)
                    .stroke(Stroke::new(1.0, t.line))
                    .corner_radius(5)
                    .inner_margin(8)
                    .show(ui, |ui| {
                        ui.set_min_width(220.0);
                        let s = &mut self.agent.settings;
                        let before = s.clone();
                        let claude = match s.claude_model.trim() {
                            "" => "Claude Code".to_string(),
                            m => format!("Claude Code · {m}"),
                        };
                        let mut other = s.clone();
                        other.kind = agent_settings::Kind::OpenAi;
                        let openai = other.label();
                        let c = ui.radio(s.kind == agent_settings::Kind::ClaudeCode, claude);
                        rows.push(("agent:switch:claude", c.rect));
                        if c.clicked() {
                            s.kind = agent_settings::Kind::ClaudeCode;
                            close = true;
                        }
                        other.kind = agent_settings::Kind::Codex;
                        let c = ui.radio(s.kind == agent_settings::Kind::Codex, other.label());
                        rows.push(("agent:switch:codex", c.rect));
                        if c.clicked() {
                            s.kind = agent_settings::Kind::Codex;
                            close = true;
                        }
                        let o = ui.radio(s.kind == agent_settings::Kind::OpenAi, openai);
                        rows.push(("agent:switch:openai", o.rect));
                        if o.clicked() {
                            s.kind = agent_settings::Kind::OpenAi;
                            close = true;
                            if agent_settings::missing(s).is_some() {
                                open_settings = true;
                            }
                        }
                        if self.agent.settings != before {
                            self.agent.out.push(AgentAct::Settings(SettingsAct::Changed));
                        }
                        ui.separator();
                        // a paired page: keep the pairing on this device for a week (opt in)
                        if let Some(d) = self.agent.device {
                            let mut on = d > 0;
                            let c = ui
                                .checkbox(&mut on, "Remember this device for 7 days")
                                .on_hover_text(
                                    "A reload pairs again by itself. The token is kept in this browser for this \
                                     page: any script from this page's site can read it. Untick, or type \
                                     `forget` where forgecad web runs, to forget it.",
                                );
                            rows.push(("agent:switch:remember", c.rect));
                            if c.changed() {
                                self.agent.out.push(AgentAct::RememberDevice(on));
                                close = true;
                            }
                        }
                        let b = ui.add(
                            egui::Button::new(egui::RichText::new("Agent settings…").color(t.ink))
                                .frame(false),
                        );
                        rows.push(("agent:switch:settings", b.rect));
                        if b.clicked() {
                            open_settings = true;
                            close = true;
                        }
                    });
            });
        if open_settings {
            self.open_agent_settings();
        }
        let r = area.response.rect;
        self.note("agent:switcher", r);
        for (n, rr) in rows {
            self.note(n, rr);
        }
        let pressed_out = ctx.input(|i| {
            i.pointer.any_pressed()
                && i.pointer
                    .interact_pos()
                    .is_some_and(|p| !r.contains(p) && !under.contains(p))
        });
        if close || pressed_out {
            self.agent.switcher = false;
        }
    }

    /// Manage > Settings, on its Agent section.
    pub fn open_agent_settings(&mut self) {
        self.prefs.open = true;
        if let Some(k) = super::settings::SECTIONS.iter().position(|x| x.0 == "Agent") {
            self.prefs.tab = k;
        }
    }

    fn agent_chat(&mut self, ui: &mut Ui, area: Rect) {
        let t = self.theme;
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(area));
        let w = area.width();
        let inner = w - 2.0 * PAD;
        let mut acts: Vec<AgentAct> = Vec::new();
        let mut toggles: Vec<String> = Vec::new();
        let mut places: Vec<(String, Rect)> = Vec::new();
        let mut hovered: Option<String> = None;
        let code_ink = if t.dark {
            Color32::from_rgb(0xC9, 0xD7, 0xF0)
        } else {
            Color32::from_rgb(0x24, 0x3B, 0x63)
        };
        for (k, e) in self.agent.entries.iter().enumerate() {
            if let Entry::User { markup: Some(m), .. } = e
                && !self.agent.thumbs.contains_key(&k)
            {
                let img = egui::ColorImage::from_rgba_unmultiplied([m.thumb.w, m.thumb.h], &m.thumb.rgba);
                let tex =
                    ui.ctx()
                        .load_texture(format!("agent-markup-{k}"), img, egui::TextureOptions::LINEAR);
                self.agent.thumbs.insert(k, tex);
            }
        }
        let jump = std::mem::take(&mut self.agent.jump);
        // a waiting question's change in words, when it's one param's value
        let headlines: std::collections::HashMap<&str, String> = self
            .agent
            .entries
            .iter()
            .filter_map(|e| match e {
                Entry::Permission {
                    id,
                    input,
                    state: Asked::Waiting,
                    ..
                } => Some((id.as_str(), self.param_headline(input)?)),
                _ => None,
            })
            .collect();
        // each entry's height when last drawn, at this width: an entry out of sight is that much
        // space, not drawn (a long chat costs what's on screen)
        let mut heights = std::mem::take(&mut self.agent.heights);
        if heights.0 != w {
            heights = (w, Vec::new());
        }
        heights.1.truncate(self.agent.entries.len());
        let a = &self.agent;
        let mut fresh = false;
        let shown = egui::ScrollArea::vertical()
            .id_salt("agent-chat")
            .auto_shrink([false, false])
            .stick_to_bottom(true)
            .show_viewport(&mut child, |ui, viewport| {
                ui.spacing_mut().item_spacing.y = 0.0;
                let origin = ui.min_rect().top();
                let near = viewport.expand2(vec2(0.0, VIRTUAL_MARGIN));
                ui.add_space(8.0);
                let mut tools_seen = 0;
                let last = a.entries.len().saturating_sub(2);
                // the entry being drawn, and where it began: its height is taken at the next
                let mut drawing: Option<(usize, f32)> = None;
                // each tool call's question, if it was asked one
                let asked_of: std::collections::HashMap<&str, Asked> = a
                    .entries
                    .iter()
                    .filter_map(|e| match e {
                        Entry::Permission { tool_use, state, .. } => Some((tool_use.as_str(), *state)),
                        _ => None,
                    })
                    .collect();
                if let Some(pr) = &a.problem {
                    problem_card(ui, &t, pr, w, &mut places);
                }
                if a.entries.is_empty() && a.problem.is_none() {
                    let what = match &a.file {
                        Some(f) if self.fg.is_some() || self.drawing.is_some() => {
                            format!("Ask for a change to {f}")
                        }
                        _ => "Ask about this project, organize its folders, or create your first part".into(),
                    };
                    if a.started_new {
                        let g =
                            ui.fonts_mut(|f| f.layout_job(plain_job(STARTED_NEW, sans(12.0), t.ink2, inner)));
                        let (r, _) = ui.allocate_exact_size(vec2(w, g.size().y + 10.0), Sense::hover());
                        ui.painter().galley(pos2(PAD, r.top() + 8.0), g, t.ink2);
                        places.push(("agent:started-new".into(), r));
                    }
                    let g = ui.fonts_mut(|f| f.layout_job(plain_job(&what, sans(13.0), t.ink2, inner)));
                    let (r, _) = ui.allocate_exact_size(vec2(w, g.size().y + 16.0), Sense::hover());
                    ui.painter().galley(pos2(PAD, r.top() + 8.0), g, t.ink2);
                }
                for (k, e) in a.entries.iter().enumerate() {
                    let y = ui.cursor().top();
                    if let Some((j, top)) = drawing.take() {
                        set_height(&mut heights.1, j, y - top);
                    }
                    // out of sight, and drawn before at this width: its space only (the last two,
                    // still changing as the turn streams, always drawn)
                    if k < last
                        && let Some(h) = heights.1.get(k).copied().filter(|h| *h > 0.0)
                    {
                        let at = y - origin;
                        if at + h < near.top() || at > near.bottom() {
                            if matches!(e, Entry::Tool { .. }) {
                                tools_seen += 1;
                            }
                            ui.add_space(h);
                            continue;
                        }
                    }
                    drawing = Some((k, y));
                    if let Some((at, said)) = &a.resumed
                        && *at == k
                    {
                        fresh |= resumed_divider(ui, &t, w, said, &mut places);
                    }
                    match e {
                        Entry::User {
                            text,
                            refs,
                            markup,
                            pictures,
                        } => {
                            ui.add_space(6.0);
                            let g = ui.fonts_mut(|f| {
                                f.layout_job(plain_job(text, sans(13.0), t.ink, inner - 16.0))
                            });
                            let chip_h = if refs.is_empty() { 0.0 } else { 22.0 };
                            let thumb = markup.as_ref().zip(a.thumbs.get(&k)).map(|(m, tex)| {
                                let s = ((inner - 16.0) / m.thumb.w as f32).min(96.0 / m.thumb.h as f32);
                                (tex.id(), vec2(m.thumb.w as f32 * s, m.thumb.h as f32 * s))
                            });
                            let thumb_h = thumb.map_or(0.0, |t| t.1.y + 6.0);
                            let pics_h = if *pictures > 0 { 18.0 } else { 0.0 };
                            let text_h = if text.is_empty() { -4.0 } else { g.size().y };
                            let (r, _) = ui.allocate_exact_size(
                                vec2(w, text_h + 14.0 + chip_h + thumb_h + pics_h),
                                Sense::hover(),
                            );
                            let card = Rect::from_min_max(pos2(PAD, r.top()), pos2(w - PAD, r.bottom()));
                            ui.painter().rect_filled(card, 6.0, t.raised);
                            ui.painter()
                                .rect_stroke(card, 6.0, Stroke::new(1.0, t.line), StrokeKind::Inside);
                            selectable(ui, pos2(card.left() + 8.0, card.top() + 7.0), g);
                            if let Some((tex, size)) = thumb {
                                let tr = Rect::from_min_size(
                                    pos2(card.left() + 8.0, card.top() + 10.0 + text_h),
                                    size,
                                );
                                let resp = ui
                                    .interact(tr, ui.scope_id().with(("agent-markup", k)), Sense::click())
                                    .on_hover_text("Show these marks over the view");
                                let whole = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
                                ui.painter().image(tex, tr, whole, Color32::WHITE);
                                let lit = resp.hovered() || a.markup.shown == Some(k);
                                ui.painter().rect_stroke(
                                    tr,
                                    3.0,
                                    Stroke::new(
                                        if lit { 1.5 } else { 1.0 },
                                        if lit { t.focus } else { t.line },
                                    ),
                                    StrokeKind::Outside,
                                );
                                if resp.clicked() {
                                    acts.push(AgentAct::ShowMarkup(k));
                                }
                                places.push((format!("agent:markup:{k}"), tr));
                            }
                            if *pictures > 0 {
                                ui.painter().text(
                                    pos2(card.left() + 8.0, card.top() + 7.0 + text_h + thumb_h + 11.0),
                                    Align2::LEFT_CENTER,
                                    pictures_word(*pictures),
                                    sans(11.0),
                                    t.ink2,
                                );
                            }
                            if !refs.is_empty() {
                                let mut x = card.left() + 8.0;
                                for id in refs {
                                    let cr = chip(
                                        ui,
                                        &t,
                                        pos2(x, card.bottom() - 25.0),
                                        id,
                                        card.right() - 8.0 - x,
                                        false,
                                    );
                                    // the pointer on it lights what it names in the view
                                    if ui.rect_contains_pointer(cr) {
                                        hovered = Some(id.clone());
                                    }
                                    x += cr.width() + 4.0;
                                }
                            }
                            places.push((format!("agent:user:{k}"), card));
                            // `put` (the selectable text) left the cursor under the text, above
                            // the chips: back under the card
                            ui.advance_cursor_after_rect(r);
                            ui.add_space(6.0);
                        }
                        Entry::Agent { text } => {
                            let top = ui.cursor().top();
                            for (j, (code, part)) in segments(text).into_iter().enumerate() {
                                if !code {
                                    for (n, (table, run)) in tables(part).into_iter().enumerate() {
                                        if table {
                                            table_grid(
                                                ui,
                                                &t,
                                                run,
                                                inner,
                                                code_ink,
                                                ("agent-table", k, j, n),
                                            );
                                            continue;
                                        }
                                        let g =
                                            ui.fonts_mut(|f| f.layout_job(rich(run, inner, t.ink, code_ink)));
                                        let (r, _) = ui
                                            .allocate_exact_size(vec2(w, g.size().y + 10.0), Sense::hover());
                                        selectable(ui, pos2(PAD, r.top() + 5.0), g);
                                    }
                                    continue;
                                }
                                // a code block: its own card, a copy button on it
                                let g = ui.fonts_mut(|f| {
                                    f.layout_job(plain_job(part, mono(12.0), code_ink, inner - 16.0))
                                });
                                let (r, _) =
                                    ui.allocate_exact_size(vec2(w, g.size().y + 16.0), Sense::hover());
                                let card = Rect::from_min_max(
                                    pos2(PAD, r.top() + 2.0),
                                    pos2(w - PAD, r.bottom() - 2.0),
                                );
                                ui.painter().rect_filled(card, 4.0, t.field);
                                selectable(ui, pos2(card.left() + 8.0, card.top() + 6.0), g);
                                let br = Rect::from_min_size(
                                    pos2(card.right() - 44.0, card.top() + 3.0),
                                    vec2(40.0, 18.0),
                                );
                                let resp = ui
                                    .interact(br, ui.scope_id().with(("agent-code", k, j)), Sense::click())
                                    .on_hover_text("Copy the code");
                                resp.widget_info(|| {
                                    egui::WidgetInfo::labeled(egui::Role::Button, true, "Copy")
                                });
                                if resp.hovered() {
                                    ui.painter().rect_filled(br, 3.0, t.hover);
                                }
                                ui.painter().text(
                                    br.center(),
                                    Align2::CENTER_CENTER,
                                    "Copy",
                                    sans(11.0),
                                    t.ink2,
                                );
                                if resp.clicked() {
                                    ui.ctx().copy_text(part.to_string());
                                }
                                places.push((format!("agent:code:{k}:{j}"), br));
                            }
                            let r = Rect::from_min_max(pos2(0.0, top), pos2(w, ui.cursor().top()));
                            places.push((format!("agent:reply:{k}"), r));
                        }
                        Entry::Tool {
                            id,
                            summary,
                            input,
                            output,
                            state,
                            ..
                        } => {
                            let open = a.open.contains(id);
                            let (r, resp) = ui.allocate_exact_size(vec2(w, TOOL_H), Sense::click());
                            if resp.hovered() {
                                ui.painter()
                                    .rect_filled(r.shrink2(vec2(PAD - 4.0, 1.0)), 3.0, t.hover);
                            }
                            if resp.clicked() {
                                toggles.push(id.clone());
                            }
                            // what the row says: while a question about it waits, that it waits (not
                            // that it happened); declined, that it didn't; running, that it is
                            let asked = asked_of.get(id.as_str()).copied();
                            let (said, ink, dot) = match (asked, state) {
                                (Some(Asked::Waiting), _) => {
                                    (format!("{} · waiting for you", present(summary)), t.warn, t.warn)
                                }
                                (Some(Asked::Denied), _) => {
                                    (not_done(summary), t.ink2, t.ink2.gamma_multiply(0.6))
                                }
                                (_, ToolState::Running) => (
                                    progressive(summary),
                                    t.ink2,
                                    t.focus.gamma_multiply(0.5 + 0.5 * pulse(ui)),
                                ),
                                (_, ToolState::Done) => (summary.clone(), t.ink2, ADD_INK),
                                (_, ToolState::Failed) => (summary.clone(), t.ink2, t.error),
                            };
                            super::fg::disclosure(ui.painter(), pos2(PAD + 4.0, r.center().y), open, ink);
                            ui.painter()
                                .circle_filled(pos2(w - PAD - 4.0, r.center().y), 3.0, dot);
                            let clip = Rect::from_min_max(
                                pos2(PAD + 14.0, r.top()),
                                pos2(w - PAD - 12.0, r.bottom()),
                            );
                            let g = ui.fonts_mut(|f| f.layout_no_wrap(said, sans(12.0), ink));
                            ui.painter().with_clip_rect(clip).galley(
                                pos2(clip.left(), r.center().y - g.size().y * 0.5),
                                g,
                                ink,
                            );
                            tools_seen += 1;
                            places.push((format!("agent:tool:{id}"), r));
                            places.push((format!("agent:tool#{tools_seen}"), r));
                            if open {
                                let mut more = None;
                                let mut body = input.trim_end().to_string();
                                if !output.trim().is_empty() {
                                    if !body.is_empty() {
                                        body.push_str("\n---\n");
                                    }
                                    // the first 40 lines, or all of them once asked
                                    let all = a.open.contains(&format!("all:{id}"));
                                    let n = output.lines().count();
                                    let out: Vec<&str> =
                                        output.trim_end().lines().take(if all { n } else { 40 }).collect();
                                    body.push_str(&out.join("\n"));
                                    more = (n > 40).then_some((n, all));
                                }
                                let g = ui.fonts_mut(|f| f.layout_job(diff_job(&body, &t, inner - 12.0)));
                                let (r, _) =
                                    ui.allocate_exact_size(vec2(w, g.size().y + 12.0), Sense::hover());
                                let card = Rect::from_min_max(
                                    pos2(PAD, r.top() + 1.0),
                                    pos2(w - PAD, r.bottom() - 3.0),
                                );
                                ui.painter().rect_filled(card, 4.0, t.field);
                                selectable(ui, pos2(card.left() + 6.0, card.top() + 4.0), g);
                                ui.advance_cursor_after_rect(r);
                                if let Some((n, all)) = more {
                                    let word = if all {
                                        "Show the first 40 lines".to_string()
                                    } else {
                                        format!("Show all {n} lines")
                                    };
                                    let (rr, resp) = ui.allocate_exact_size(vec2(w, 20.0), Sense::click());
                                    resp.widget_info(|| {
                                        egui::WidgetInfo::labeled(egui::Role::Button, true, &word)
                                    });
                                    let ink = if resp.hovered() { t.ink } else { t.focus };
                                    ui.painter().text(
                                        pos2(PAD + 6.0, rr.center().y),
                                        Align2::LEFT_CENTER,
                                        word,
                                        sans(11.0),
                                        ink,
                                    );
                                    places.push((format!("agent:tool-all:{id}"), rr));
                                    if resp.clicked() {
                                        toggles.push(format!("all:{id}"));
                                    }
                                }
                            }
                        }
                        Entry::Diff {
                            turn,
                            index,
                            created,
                            file,
                            lines,
                            added,
                            removed,
                            undone,
                        } => {
                            ui.add_space(4.0);
                            let key = format!("diff:{turn}:{index}");
                            let expanded = a.open.contains(&key);
                            let shown = if expanded { lines.len().min(60) } else { 0 };
                            let more = lines.len() - shown;
                            let h = if expanded {
                                26.0 + shown as f32 * DIFF_ROW + if more > 0 { DIFF_ROW } else { 0.0 } + 6.0
                            } else {
                                32.0
                            };
                            let (r, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
                            let card = Rect::from_min_max(pos2(PAD, r.top()), pos2(w - PAD, r.bottom()));
                            let p = ui.painter().clone();
                            p.rect_filled(card, 5.0, t.field);
                            p.rect_stroke(card, 5.0, Stroke::new(1.0, t.line), StrokeKind::Inside);
                            let head = Rect::from_min_size(card.min, vec2(card.width(), 26.0));
                            let toggle =
                                Rect::from_min_max(head.min, pos2(head.right() - 56.0, head.bottom()));
                            let toggle_resp = ui
                                .interact(
                                    toggle,
                                    ui.scope_id().with(("agent-diff-toggle", *turn, *index)),
                                    Sense::click(),
                                )
                                .on_hover_text(if expanded {
                                    "Hide code changes"
                                } else {
                                    "Show code changes"
                                });
                            toggle_resp.widget_info(|| {
                                egui::WidgetInfo::labeled(
                                    egui::Role::Button,
                                    true,
                                    if expanded {
                                        "Hide code changes"
                                    } else {
                                        "Show code changes"
                                    },
                                )
                            });
                            places.push((format!("agent:diff:toggle:{turn}:{index}"), toggle));
                            if toggle_resp.clicked() {
                                toggles.push(key);
                            }
                            icons::draw(
                                &p,
                                pos2(head.left() + 5.0, head.center().y - 6.0),
                                12.0,
                                if expanded { "chevron" } else { "right" },
                                1.5,
                                t.ink2,
                            );
                            let name = p.text(
                                pos2(head.left() + 22.0, head.center().y),
                                Align2::LEFT_CENTER,
                                file,
                                medium(12.0),
                                t.ink,
                            );
                            let name = if *created {
                                p.text(
                                    pos2(name.right() + 6.0, head.center().y),
                                    Align2::LEFT_CENTER,
                                    "new",
                                    sans(11.0),
                                    t.ink2,
                                )
                            } else {
                                name
                            };
                            let plus = p.text(
                                pos2(name.right() + 8.0, head.center().y),
                                Align2::LEFT_CENTER,
                                format!("+{added}"),
                                mono(11.0),
                                ADD_INK,
                            );
                            p.text(
                                pos2(plus.right() + 5.0, head.center().y),
                                Align2::LEFT_CENTER,
                                format!("−{removed}"),
                                mono(11.0),
                                DEL_INK,
                            );
                            let word = if *undone { "Redo" } else { "Undo" };
                            let bw = 48.0;
                            let br = Rect::from_min_size(
                                pos2(head.right() - bw - 4.0, head.top() + 3.0),
                                vec2(bw, 20.0),
                            );
                            let resp = ui.interact(
                                br,
                                ui.scope_id().with(("agent-undo", *turn, *index)),
                                Sense::click(),
                            );
                            resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, word));
                            // from before a resume: it undoes only over what the agent left
                            let resp = if !*undone && a.resumed.as_ref().is_some_and(|(at, _)| k < *at) {
                                resp.on_hover_text(format!(
                                    "Undoes this change while {file} is as the agent left it."
                                ))
                            } else {
                                resp
                            };
                            p.rect_filled(br, 3.0, if resp.hovered() { t.hover } else { t.raised });
                            p.rect_stroke(br, 3.0, Stroke::new(1.0, t.line), StrokeKind::Inside);
                            p.text(br.center(), Align2::CENTER_CENTER, word, sans(12.0), t.ink);
                            places.push((format!("agent:undo:{turn}:{index}"), br));
                            places.push((format!("agent:diff:{turn}:{index}"), card));
                            if resp.clicked() {
                                acts.push(if *undone {
                                    AgentAct::Redo(*turn, *index)
                                } else {
                                    AgentAct::Undo(*turn, *index)
                                });
                            }
                            if !expanded {
                                ui.add_space(4.0);
                                continue;
                            }
                            p.hline(
                                card.left()..=card.right(),
                                head.bottom(),
                                Stroke::new(1.0, t.line),
                            );
                            let body = p.with_clip_rect(card.shrink(1.0));
                            let mut y = head.bottom() + 3.0;
                            let dim = if *undone { 0.45 } else { 1.0 };
                            for l in &lines[..shown] {
                                let row = Rect::from_min_size(
                                    pos2(card.left() + 1.0, y),
                                    vec2(card.width() - 2.0, DIFF_ROW),
                                );
                                let (mark, text, ink) = match l {
                                    Line::Same(s) => (" ", s.as_str(), t.ink2),
                                    Line::Add(s) => {
                                        body.rect_filled(row, 0.0, ADD_BG.gamma_multiply(dim));
                                        ("+", s.as_str(), ADD_INK)
                                    }
                                    Line::Del(s) => {
                                        body.rect_filled(row, 0.0, DEL_BG.gamma_multiply(dim));
                                        ("−", s.as_str(), DEL_INK)
                                    }
                                    Line::Skip(n) => {
                                        body.text(
                                            pos2(card.left() + 8.0, row.center().y),
                                            Align2::LEFT_CENTER,
                                            format!("{n} unchanged lines"),
                                            sans(11.0),
                                            t.ink2.gamma_multiply(0.8),
                                        );
                                        y += DIFF_ROW;
                                        continue;
                                    }
                                };
                                body.text(
                                    pos2(card.left() + 6.0, row.center().y),
                                    Align2::LEFT_CENTER,
                                    mark,
                                    mono(11.0),
                                    ink.gamma_multiply(dim),
                                );
                                body.text(
                                    pos2(card.left() + 18.0, row.center().y),
                                    Align2::LEFT_CENTER,
                                    text,
                                    mono(11.0),
                                    ink.gamma_multiply(dim),
                                );
                                y += DIFF_ROW;
                            }
                            if more > 0 {
                                body.text(
                                    pos2(card.left() + 8.0, y + DIFF_ROW * 0.5),
                                    Align2::LEFT_CENTER,
                                    format!("… {more} more lines"),
                                    sans(11.0),
                                    t.ink2,
                                );
                            }
                            ui.add_space(4.0);
                        }
                        Entry::Permission {
                            id,
                            summary,
                            input,
                            always,
                            state,
                            ..
                        } => {
                            let first = a.waiting() == Some(id.as_str());
                            permission_card(
                                ui,
                                &t,
                                Card {
                                    id,
                                    summary,
                                    input,
                                    always: always.as_deref(),
                                    state: *state,
                                    first,
                                    headline: headlines.get(id.as_str()).cloned(),
                                },
                                w,
                                &mut places,
                            );
                        }
                        Entry::Note { text, error } => {
                            let ink = if *error { t.error } else { t.ink2 };
                            let g = ui.fonts_mut(|f| f.layout_job(plain_job(text, sans(12.0), ink, inner)));
                            let (r, _) = ui.allocate_exact_size(vec2(w, g.size().y + 8.0), Sense::hover());
                            selectable(ui, pos2(PAD, r.top() + 4.0), g);
                            places.push((format!("agent:note:{k}"), r));
                        }
                    }
                }
                if let Some((j, top)) = drawing.take() {
                    set_height(&mut heights.1, j, ui.cursor().top() - top);
                }
                if let Some((at, said)) = &a.resumed
                    && *at == a.entries.len()
                {
                    fresh |= resumed_divider(ui, &t, w, said, &mut places);
                }
                if let Some(dir) = &a.trust_ask {
                    trust_card(ui, &t, dir, w, &mut places);
                }
                if a.running && a.trust_ask.is_none() {
                    let now = ui.input(|i| i.time);
                    // its time so far and its tools, as a terminal's spinner says them
                    let secs = a.turn_started.map_or(0.0, |t0| now - t0);
                    let tools = a
                        .entries
                        .iter()
                        .rev()
                        .take_while(|e| !matches!(e, Entry::User { .. }))
                        .filter(|e| matches!(e, Entry::Tool { .. }))
                        .count();
                    let mut said = format!("Working · {}", duration(secs));
                    if tools > 0 {
                        said.push_str(&format!(" · {tools} tool{}", if tools == 1 { "" } else { "s" }));
                    }
                    if a.queued > 0 {
                        said.push_str(&format!(" · {} queued", a.queued));
                    }
                    // §7's loading state: a spinner and what it's doing
                    let r = ui
                        .scope_builder(
                            egui::UiBuilder::new().max_rect(Rect::from_min_size(
                                pos2(PAD, ui.cursor().top() + 4.0),
                                vec2(w - PAD, 18.0),
                            )),
                            |ui| super::ux::loading(ui, &t, &said),
                        )
                        .response
                        .rect;
                    ui.advance_cursor_after_rect(r.expand2(vec2(0.0, 4.0)));
                    places.push(("agent:working".into(), r));
                    // what's queued behind the turn: each to take back and change, or drop
                    for (q, text) in a.queue.iter().enumerate() {
                        if let Some(act) = queued_row(ui, &t, w, q, text, &mut places) {
                            acts.push(act);
                        }
                    }
                } else if let Some((secs, tin, tout, cost)) = a.usage
                    && !a.entries.is_empty()
                {
                    // what the last turn took, faint; what it cost, when it's paid per token
                    let mut said = format!(
                        "{} · {} tokens in · {} out",
                        duration(secs),
                        thousands(tin),
                        thousands(tout)
                    );
                    if a.api_keyed && cost > 0.0 {
                        said.push_str(&format!(" · ${cost:.2}"));
                    }
                    let (r, _) = ui.allocate_exact_size(vec2(w, 18.0), Sense::hover());
                    ui.painter().text(
                        pos2(PAD, r.center().y),
                        Align2::LEFT_CENTER,
                        said,
                        sans(11.0),
                        t.ink2.gamma_multiply(0.8),
                    );
                    places.push(("agent:usage".into(), r));
                }
                // the plan's limit: amber when reached, faint when close
                if let Some((reached, resets)) = a.limit {
                    let said = limit_words(reached, resets, forge_history::now() as f64);
                    let (r, _) = ui.allocate_exact_size(vec2(w, 18.0), Sense::hover());
                    let ink = if reached {
                        t.warn
                    } else {
                        t.ink2.gamma_multiply(0.8)
                    };
                    ui.painter().text(
                        pos2(PAD, r.center().y),
                        Align2::LEFT_CENTER,
                        said,
                        sans(11.0),
                        ink,
                    );
                    places.push(("agent:limit".into(), r));
                }
                ui.add_space(10.0);
                // "Jump to latest": to the end, where it sticks again
                if jump {
                    ui.scroll_to_cursor_animation(
                        Some(egui::Align::BOTTOM),
                        egui::style::ScrollAnimation::none(),
                    );
                }
            });
        if hovered.is_some() {
            self.agent.hover_ref = hovered;
        }
        for id in toggles {
            if !self.agent.open.remove(&id) {
                self.agent.open.insert(id);
            }
        }
        // where the reader is: at the end, or scrolled up (a chip takes them back), and whether a
        // waiting question's buttons are in sight (Enter answers only one that is)
        self.agent.heights = heights;
        let view = shown.inner_rect;
        let at_end = shown.state.clamped_offset().y + view.height() >= shown.content_size.y - 8.0;
        let waiting = self.agent.waiting().map(str::to_string);
        self.agent.card_visible = waiting.as_ref().is_none_or(|id| {
            let name = format!("agent:perm:{id}:allow");
            places
                .iter()
                .find(|p| p.0 == name)
                .is_some_and(|p| view.contains_rect(p.1))
        });
        if !at_end || !self.agent.card_visible {
            let word = if self.agent.card_visible {
                "Jump to latest"
            } else {
                "Permission needed"
            };
            let g = ui.fonts_mut(|f| f.layout_no_wrap(word.to_string(), medium(12.0), t.ink));
            // the text, then a down chevron (the app's font has no arrows)
            let size = g.size() + vec2(36.0, 8.0);
            let jr = Rect::from_min_size(
                pos2(view.center().x - size.x * 0.5, view.bottom() - size.y - 8.0),
                size,
            );
            let resp = ui.interact(jr, ui.scope_id().with("agent-jump"), Sense::click());
            resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, word));
            let fill = if self.agent.card_visible { t.raised } else { t.warn };
            let ink = if self.agent.card_visible {
                t.ink
            } else {
                super::theme::ON_FOCUS
            };
            ui.painter().rect_filled(
                jr,
                12.0,
                if resp.hovered() {
                    fill.gamma_multiply(0.9)
                } else {
                    fill
                },
            );
            ui.painter()
                .rect_stroke(jr, 12.0, Stroke::new(1.0, t.line), StrokeKind::Inside);
            ui.painter().galley(jr.min + vec2(10.0, 4.0), g, ink);
            down_chevron(ui.painter(), pos2(jr.right() - 14.0, jr.center().y), ink);
            places.push(("agent:jump".into(), jr));
            if resp.clicked() {
                self.agent.jump = true;
                ui.ctx().request_repaint();
            }
        }
        if let Some(fix) = places.iter().find(|p| p.0 == "agent:copy-fix").map(|p| p.1)
            && ui
                .interact(fix, ui.scope_id().with("agent-copy"), Sense::click())
                .clicked()
            && let Some(cmd) = self.agent.problem.as_ref().and_then(|p| p.fix.clone())
        {
            ui.ctx().copy_text(cmd);
            self.status = "Copied the command".into();
        }
        let mut answers = Vec::new();
        for e in &self.agent.entries {
            if let Entry::Permission {
                id,
                state: Asked::Waiting,
                ..
            } = e
            {
                for (word, ans) in [
                    ("allow", Answer::Allow),
                    ("always", Answer::Always),
                    ("deny", Answer::Deny),
                ] {
                    let name = format!("agent:perm:{id}:{word}");
                    if let Some(r) = places.iter().find(|p| p.0 == name).map(|p| p.1)
                        && ui
                            .interact(r, ui.scope_id().with(&name), Sense::click())
                            .clicked()
                    {
                        answers.push((id.clone(), ans));
                    }
                }
            }
        }
        for (id, ans) in answers {
            self.agent.answer(&id, ans);
        }
        if let Some(dir) = self.agent.trust_ask.clone() {
            for (name, yes) in [("agent:trust:yes", true), ("agent:trust:no", false)] {
                if let Some(r) = places.iter().find(|p| p.0 == name).map(|p| p.1)
                    && ui.interact(r, ui.scope_id().with(name), Sense::click()).clicked()
                {
                    acts.push(AgentAct::Trust {
                        dir: dir.clone(),
                        yes,
                    });
                }
            }
        }
        self.places.extend(places);
        if fresh {
            self.agent.new_chat();
        }
        for a in acts {
            // the thumbnail again hides its marks
            if let AgentAct::ShowMarkup(k) = a {
                if self.agent.markup.shown == Some(k) {
                    self.agent.markup.shown = None;
                    continue;
                }
                self.agent.markup.shown = Some(k);
            }
            self.agent.out.push(a);
        }
    }

    /// The mode chip's menu, under it: each mode, what it means, the one in force ticked.
    fn mode_menu(&mut self, ctx: &egui::Context, under: Rect) {
        let t = self.theme;
        let mut picked = None;
        let mut own_picked = false;
        // the person's own settings' mode, to go back to (not in a page: it may not turn a
        // bypass back on)
        let own = if cfg!(target_arch = "wasm32") {
            None
        } else {
            self.agent
                .own_mode
                .clone()
                .map(|f| (format!("Your settings: {}", AgentTab::mode_word(&f)), f))
        };
        const OWN_MEANS: &str = "As your Claude Code settings say";
        // as wide as its longest line
        let w = Mode::ALL
            .iter()
            .map(|m| m.means().to_string())
            .chain(own.iter().flat_map(|(w, _)| [w.clone(), OWN_MEANS.to_string()]))
            .map(|line| ctx.fonts_mut(|f| f.layout_no_wrap(line, sans(11.0), t.ink).size().x))
            .fold(0.0, f32::max)
            + 28.0;
        let at = pos2((under.right() - w).max(4.0), under.bottom() + 4.0);
        let area = egui::Area::new(egui::Id::unique("agent-mode-menu"))
            .order(egui::Order::Foreground)
            .fixed_pos(at)
            .show(ctx, |ui| {
                let row = 40.0;
                let rows = 3 + own.is_some() as usize;
                let r = Rect::from_min_size(at, vec2(w, row * rows as f32 + 8.0));
                let (_, resp) = ui.allocate_exact_size(r.size(), Sense::hover());
                let p = ui.painter();
                p.rect_filled(r, 6.0, t.raised);
                p.rect_stroke(r, 6.0, Stroke::new(1.0, t.line), StrokeKind::Inside);
                for (k, m) in Mode::ALL.into_iter().enumerate() {
                    let rr = Rect::from_min_size(
                        pos2(r.left() + 4.0, r.top() + 4.0 + k as f32 * row),
                        vec2(w - 8.0, row),
                    );
                    let hit = ui.interact(rr, ui.scope_id().with(("agent-mode-row", k)), Sense::click());
                    hit.widget_info(|| {
                        egui::WidgetInfo::selected(
                            egui::Role::RadioButton,
                            true,
                            self.agent.mode == m,
                            m.word(),
                        )
                    });
                    let on = self.agent.mode == m && self.agent.mode_other.is_none();
                    if on {
                        p.rect_filled(rr, 4.0, t.on);
                    } else if hit.hovered() {
                        p.rect_filled(rr, 4.0, t.hover);
                    }
                    let ink = if on { t.on_ink } else { t.ink };
                    p.text(
                        pos2(rr.left() + 10.0, rr.top() + 12.0),
                        Align2::LEFT_CENTER,
                        m.word(),
                        medium(12.0),
                        ink,
                    );
                    p.text(
                        pos2(rr.left() + 10.0, rr.top() + 28.0),
                        Align2::LEFT_CENTER,
                        m.means(),
                        sans(11.0),
                        if on { ink } else { t.ink2 },
                    );
                    self.places.push((format!("agent:mode:{}", m.flag()), rr));
                    if hit.clicked() {
                        picked = Some(m);
                    }
                }
                if let Some((word, flag)) = &own {
                    let rr = Rect::from_min_size(
                        pos2(r.left() + 4.0, r.top() + 4.0 + 3.0 * row),
                        vec2(w - 8.0, row),
                    );
                    let on = self.agent.mode_other.as_ref() == Some(flag);
                    let hit = ui.interact(rr, ui.scope_id().with("agent-mode-own"), Sense::click());
                    hit.widget_info(|| egui::WidgetInfo::selected(egui::Role::RadioButton, true, on, word));
                    if on {
                        p.rect_filled(rr, 4.0, t.on);
                    } else if hit.hovered() {
                        p.rect_filled(rr, 4.0, t.hover);
                    }
                    let ink = if on { t.on_ink } else { t.ink };
                    p.line_segment(
                        [
                            pos2(rr.left() + 6.0, rr.top() - 0.5),
                            pos2(rr.right() - 6.0, rr.top() - 0.5),
                        ],
                        Stroke::new(1.0, t.line),
                    );
                    p.text(
                        pos2(rr.left() + 10.0, rr.top() + 12.0),
                        Align2::LEFT_CENTER,
                        word,
                        medium(12.0),
                        ink,
                    );
                    p.text(
                        pos2(rr.left() + 10.0, rr.top() + 28.0),
                        Align2::LEFT_CENTER,
                        OWN_MEANS,
                        sans(11.0),
                        if on { ink } else { t.ink2 },
                    );
                    self.places.push(("agent:mode:own".into(), rr));
                    own_picked = hit.clicked();
                }
                resp
            });
        if own_picked {
            self.agent.set_own_mode();
            self.agent.mode_menu = false;
        } else if let Some(m) = picked {
            self.agent.set_mode(m);
            self.agent.mode_menu = false;
        } else if area.inner.clicked_elsewhere()
            && !ctx.input(|i| i.pointer.interact_pos().is_some_and(|q| under.contains(q)))
        {
            self.agent.mode_menu = false;
        }
    }

    /// A param in words, and the unit its value is written in: "extrude1 depth" and " mm" for the
    /// key of the dimension it drives; the key itself, and its value's unit, otherwise.
    fn param_words(&self, key: &str) -> (String, &'static str) {
        use forge_lang::values::Dim;
        let unit = |d: Dim| match d {
            Dim::Length => match self.units {
                "in" => " in",
                "cm" => " cm",
                "m" => " m",
                "ft" => " ft",
                _ => " mm",
            },
            Dim::Angle => "°",
            _ => "",
        };
        let Some(fg) = &self.fg else {
            return (key.to_string(), "");
        };
        for f in &fg.feats {
            for d in &f.dims {
                if let super::fg::From::Driving(k) = &d.from
                    && k.to_string() == key
                {
                    return (format!("{} {}", f.id, d.what), unit(d.dim));
                }
            }
        }
        let u = fg
            .params
            .iter()
            .find(|p| p.key.to_string() == key)
            .and_then(|p| p.value)
            .map_or("", |(_, d)| unit(d));
        (key.to_string(), u)
    }

    /// An edit of one param's value, in words: "extrude1 depth from 12 to 14 mm".
    fn param_headline(&self, input: &str) -> Option<String> {
        let (key, old, new) = param_change(input)?;
        let (words, unit) = self.param_words(&key);
        // in words: the app's font has no arrows
        Some(format!("{words} from {old} to {new}{unit}"))
    }

    /// Everything `@` can name now: the open part's features and params, then the project's
    /// Forge files by their path in it.
    fn mention_candidates(&self) -> Vec<Candidate> {
        let mut all = Vec::new();
        if let Some(fg) = &self.fg {
            all.extend(fg.feats.iter().map(|f| Candidate {
                name: f.id.clone(),
                what: f.kind.clone(),
                tip: String::new(),
                ..Default::default()
            }));
            all.extend(fg.params.iter().map(|p| {
                let key = p.key.to_string();
                let (words, unit) = self.param_words(&key);
                // the dimension it drives, its value with its unit: "depth = 12 mm"
                let what = words.rsplit_once(' ').map_or("param", |w| w.1);
                Candidate {
                    name: key,
                    what: format!("{what} = {}{unit}", one_line(&p.text, 20)),
                    tip: if p.used_by.is_empty() {
                        String::new()
                    } else {
                        format!("Drives {}", p.used_by.join(", "))
                    },
                    ..Default::default()
                }
            }));
        }
        if let Some(root) = &self.agent.project {
            let root = std::path::Path::new(root);
            all.extend(
                crate::agent::project::files(root)
                    .into_iter()
                    .filter(|p| p.extension().is_some_and(|e| e == "fg"))
                    .filter_map(|p| p.strip_prefix(root).ok().map(|r| r.display().to_string()))
                    .map(|name| Candidate {
                        name,
                        what: "file".into(),
                        tip: String::new(),
                        ..Default::default()
                    }),
            );
        }
        all
    }

    fn agent_input(&mut self, ui: &mut Ui, area: Rect) {
        let t = self.theme;
        let p = ui.painter().clone();
        let box_r = Rect::from_min_max(
            area.min + vec2(PAD - 2.0, 4.0),
            area.max - vec2(PAD - 2.0, PAD - 2.0),
        );
        let id = egui::Id::unique(INPUT_ID);
        if std::mem::take(&mut self.agent.focus_input) {
            ui.memory_mut(|m| m.request_focus(id));
        }
        let focused = ui.memory(|m| m.has_focus(id));
        p.rect_filled(box_r, 6.0, t.field);
        p.rect_stroke(
            box_r,
            6.0,
            Stroke::new(
                if focused { 1.5 } else { 1.0 },
                if focused { t.focus } else { t.line },
            ),
            StrokeKind::Inside,
        );
        let mut y = box_r.top() + 4.0;
        // what's attached, each with its ×; then the selection, offered (outlined) until its ×
        let offered = self.agent.suggested().map(str::to_string);
        if !self.agent.refs.is_empty() || offered.is_some() || !self.agent.pictures.is_empty() {
            let mut x = box_r.left() + 6.0;
            // the pictures pasted or dropped, each with its ×
            let mut unpick = None;
            for (k, pic) in self.agent.pictures.iter().enumerate() {
                let kind = match pic.ext() {
                    "jpeg" => "JPG".to_string(),
                    e => e.to_uppercase(),
                };
                let label = format!("Picture {} · {kind}", k + 1);
                let thumb = picture_thumb(ui.ctx(), pic);
                let cr = picture_chip(ui, &t, pos2(x, y), &label, thumb, box_r.right() - 6.0 - x);
                let xr = Rect::from_min_size(pos2(cr.right() - 16.0, cr.top()), vec2(16.0, cr.height()));
                let resp = ui
                    .interact(xr, ui.scope_id().with(("agent-unpicture", k)), Sense::click())
                    .on_hover_text("Leave this picture out");
                resp.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::Role::Button, true, format!("Remove {label}"))
                });
                if resp.clicked() {
                    unpick = Some(k);
                }
                self.places.push((format!("agent:picture:{k}"), cr));
                self.places.push((format!("agent:picture:{k}:x"), xr));
                x += cr.width() + 4.0;
            }
            if let Some(k) = unpick {
                self.agent.pictures.remove(k);
            }
            let mut drop = None;
            for (k, r) in self.agent.refs.iter().enumerate() {
                let cr = chip(ui, &t, pos2(x, y), r, box_r.right() - 6.0 - x, true);
                if ui.rect_contains_pointer(cr) {
                    self.agent.hover_ref = Some(r.clone());
                }
                let xr = Rect::from_min_size(pos2(cr.right() - 16.0, cr.top()), vec2(16.0, cr.height()));
                if ui
                    .interact(xr, ui.scope_id().with(("agent-unref", k)), Sense::click())
                    .clicked()
                {
                    drop = Some(k);
                }
                self.places.push((format!("agent:ref:{r}"), cr));
                x += cr.width() + 4.0;
            }
            if let Some(k) = drop {
                self.agent.refs.remove(k);
            }
            if let Some(s) = &offered {
                let room = box_r.right() - 6.0 - x;
                // several selected: one chip saying how many, each named on its hover
                let all = self.agent.suggested_all();
                let label = match all.len() {
                    0 | 1 => format!("@{s}"),
                    n => format!("{n} selected"),
                };
                let cr = offered_chip(ui, &t, pos2(x, y), &label, room);
                let xr = Rect::from_min_size(pos2(cr.right() - 16.0, cr.top()), vec2(16.0, cr.height()));
                let tip = if all.len() > 1 {
                    format!(
                        "{} selected · go with your message; × to leave them out:\n{}",
                        all.len(),
                        all.iter().map(|a| describe(a)).collect::<Vec<_>>().join("\n")
                    )
                } else {
                    format!("{} · goes with your message; × to leave it out", describe(s))
                };
                let resp = ui
                    .interact(cr, ui.scope_id().with("agent-offered"), Sense::hover())
                    .on_hover_text(tip);
                if resp.hovered() {
                    self.agent.hover_ref = Some(s.clone());
                }
                resp.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::Role::Button, true, format!("Selected: {s}"))
                });
                if ui
                    .interact(xr, ui.scope_id().with("agent-offered-x"), Sense::click())
                    .clicked()
                {
                    self.agent.dismissed = Some(s.clone());
                }
                self.places.push(("agent:offered".into(), cr));
                self.places.push(("agent:offered:x".into(), xr));
            }
            y += 26.0;
        }
        // the `/` list (a command, the box's first word) or the `@` list (what the word after
        // `@` can name), kept while it's typed
        let word = slash_word(&self.agent.input)
            .map(|w| ('/', w.to_string()))
            .or_else(|| mention_word(&self.agent.input).map(|w| ('@', w.to_string())));
        match word {
            Some((sigil, w))
                if focused && self.agent.mention_off.as_deref() != Some(self.agent.input.as_str()) =>
            {
                // gathered once as the list opens (a listing of the project's files), then filtered
                let (all, at) = match self.agent.mention.take() {
                    Some(m) if m.sigil == sigil && m.word == w => (m.all, m.at),
                    Some(m) if m.sigil == sigil => (m.all, 0),
                    _ if sigil == '/' => (command_candidates(&self.agent.commands), 0),
                    _ => (self.mention_candidates(), 0),
                };
                let items = matches(&all, &w);
                let at = at.min(items.len().saturating_sub(1));
                // kept with nothing to show too, so the next key doesn't gather it all again
                self.agent.mention = Some(Mention {
                    sigil,
                    word: w,
                    items,
                    at,
                    all,
                });
            }
            _ => self.agent.mention = None,
        }
        if self.agent.mention_off.as_deref() != Some(self.agent.input.as_str()) {
            self.agent.mention_off = None;
        }
        let listing = self.agent.mention.as_ref().is_some_and(|m| !m.items.is_empty());
        let mut took = false;
        // a command picked with Enter runs; with Tab it's only written out (for its argument)
        let mut run = false;
        if focused
            && let Some(m) = &mut self.agent.mention
            && !m.items.is_empty()
        {
            let n = m.items.len();
            let command = m.sigil == '/';
            ui.input_mut(|i| {
                if i.consume_key(Modifiers::NONE, Key::ArrowDown) {
                    m.at = (m.at + 1) % n;
                }
                if i.consume_key(Modifiers::NONE, Key::ArrowUp) {
                    m.at = (m.at + n - 1) % n;
                }
                let enter = i.consume_key(Modifiers::NONE, Key::Enter);
                run = enter && command;
                took = enter || i.consume_key(Modifiers::NONE, Key::Tab);
            });
        }
        // ↑ and ↓ in an empty box (or one holding a message they brought back): the messages
        // sent before, as in a terminal; anything typed and they're the cursor's again
        if focused && !listing {
            let sent: Vec<&str> = self
                .agent
                .entries
                .iter()
                .filter_map(|e| match e {
                    Entry::User { text, .. } if !text.is_empty() => Some(text.as_str()),
                    _ => None,
                })
                .collect();
            let holding = match self.agent.recall {
                None => self.agent.input.is_empty(),
                Some(k) => sent.len() > k && sent[sent.len() - 1 - k] == self.agent.input,
            };
            if holding && !sent.is_empty() {
                let (up, down) = ui.input_mut(|i| {
                    (
                        i.consume_key(Modifiers::NONE, Key::ArrowUp),
                        i.consume_key(Modifiers::NONE, Key::ArrowDown),
                    )
                });
                let next = match (up, down, self.agent.recall) {
                    (true, _, None) => Some(Some(0)),
                    (true, _, Some(k)) => Some(Some((k + 1).min(sent.len() - 1))),
                    (_, true, Some(0)) => Some(None),
                    (_, true, Some(k)) => Some(Some(k - 1)),
                    _ => None,
                };
                if let Some(next) = next {
                    self.agent.recall = next;
                    self.agent.input = next.map_or(String::new(), |k| sent[sent.len() - 1 - k].to_string());
                    if let Some(mut st) = egui::TextEdit::load_state(ui.ctx(), id) {
                        let end = egui::text::CCursor::new(self.agent.input.chars().count());
                        st.cursor.set_char_range(Some(egui::text::CCursorRange::one(end)));
                        st.store(ui.ctx(), id);
                    }
                }
            } else {
                self.agent.recall = None;
            }
        }
        if took {
            self.agent.take_mention();
            if run {
                self.agent.input = self.agent.input.trim_end().to_string();
                self.agent.send();
            }
            // the text changed under the editor: its cursor to the end
            if let Some(mut st) = egui::TextEdit::load_state(ui.ctx(), id) {
                let end = egui::text::CCursor::new(self.agent.input.chars().count());
                st.cursor.set_char_range(Some(egui::text::CCursorRange::one(end)));
                st.store(ui.ctx(), id);
            }
        }
        let shown = self
            .agent
            .mention
            .as_ref()
            .filter(|m| !m.items.is_empty())
            .map(|m| (m.items.clone(), m.at));
        if let Some((items, at_row)) = shown {
            let m = Mention {
                items,
                at: at_row,
                sigil: self.agent.mention.as_ref().map_or('@', |m| m.sigil),
                ..Default::default()
            };
            let row = 22.0;
            let h = m.items.len() as f32 * row + 8.0;
            let at = pos2(box_r.left(), box_r.top() - h - 4.0);
            let mut picked = None;
            egui::Area::new(egui::Id::unique("agent-mention"))
                .order(egui::Order::Foreground)
                .fixed_pos(at)
                .show(ui.ctx(), |ui| {
                    let r = Rect::from_min_size(at, vec2(box_r.width(), h));
                    let p = ui.painter();
                    p.rect_filled(r, 6.0, t.raised);
                    p.rect_stroke(r, 6.0, Stroke::new(1.0, t.line), StrokeKind::Inside);
                    for (k, c) in m.items.iter().enumerate() {
                        let rr = Rect::from_min_size(
                            pos2(r.left() + 4.0, r.top() + 4.0 + k as f32 * row),
                            vec2(r.width() - 8.0, row),
                        );
                        let resp =
                            ui.interact(rr, ui.scope_id().with(("agent-mention-row", k)), Sense::click());
                        resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &c.name));
                        // the whole of what it is (the row may cut it), or what a param drives
                        let resp = resp.on_hover_text(if c.tip.is_empty() { &c.what } else { &c.tip });
                        if k == m.at || resp.hovered() {
                            p.rect_filled(rr, 4.0, if k == m.at { t.on } else { t.hover });
                        }
                        // the name first, whole; what it is in what's left, cut to one line
                        let ink = if k == m.at { t.on_ink } else { t.ink };
                        let name = ui.fonts_mut(|f| {
                            f.layout_no_wrap(format!("{}{}", m.sigil, c.name), mono(12.0), ink)
                        });
                        // whose it is, faint, at the right
                        let tag = (!c.source.is_empty()).then(|| {
                            ui.fonts_mut(|f| f.layout_no_wrap(c.source.to_string(), sans(10.0), t.ink2))
                        });
                        let tag_w = tag.as_ref().map_or(0.0, |g| g.size().x + 10.0);
                        let room = (rr.width() - name.size().x - 28.0 - tag_w).max(0.0);
                        let mut job = plain_job(&c.what, sans(11.0), t.ink2, room);
                        job.wrap.max_rows = 1;
                        job.wrap.break_anywhere = true;
                        job.wrap.overflow_character = Some('…');
                        let what = ui.fonts_mut(|f| f.layout_job(job));
                        p.with_clip_rect(rr).galley(
                            pos2(rr.left() + 8.0, rr.center().y - name.size().y * 0.5),
                            name,
                            ink,
                        );
                        if let Some(g) = tag {
                            let tr = Rect::from_min_size(
                                pos2(rr.right() - 4.0 - g.size().x - 8.0, rr.center().y - 8.0),
                                vec2(g.size().x + 8.0, 16.0),
                            );
                            p.rect_stroke(tr, 8.0, Stroke::new(1.0, t.line), StrokeKind::Inside);
                            p.galley(pos2(tr.left() + 4.0, tr.center().y - g.size().y * 0.5), g, t.ink2);
                        }
                        if room > 24.0 {
                            p.galley(
                                pos2(
                                    rr.right() - 8.0 - tag_w - what.size().x,
                                    rr.center().y - what.size().y * 0.5,
                                ),
                                what,
                                t.ink2,
                            );
                        }
                        if resp.clicked() {
                            picked = Some(k);
                        }
                        self.places.push((format!("agent:mention:{}", c.name), rr));
                    }
                });
            if let Some(k) = picked {
                let command = self.agent.mention.as_ref().is_some_and(|m| m.sigil == '/');
                if let Some(m) = &mut self.agent.mention {
                    m.at = k;
                }
                self.agent.take_mention();
                // a command clicked runs, as with Enter
                if command {
                    self.agent.input = self.agent.input.trim_end().to_string();
                    self.agent.send();
                }
            }
        }
        let send = focused && ui.input(|i| i.events.iter().any(|event| {
            matches!(event, egui::Event::Key { key: Key::Enter, pressed: true, modifiers, .. } if !modifiers.shift)
        }));
        if send {
            ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter));
        }
        let text_r = Rect::from_min_max(
            pos2(box_r.left() + 4.0, y),
            pos2(box_r.right() - 4.0, box_r.bottom() - 28.0),
        );
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(text_r));
        child.set_clip_rect(text_r.intersect(ui.clip_rect()));
        let hint = if self.agent.running {
            "Send a follow-up; it runs after this turn"
        } else {
            "Ask the agent"
        };
        let mut layouter =
            |ui: &Ui, text: &dyn egui::TextBuffer, width: f32| input_galley(ui, text.as_str(), width);
        let scroll = egui::ScrollArea::vertical()
            .id_salt("agent-input-scroll")
            .max_height(text_r.height())
            .min_scrolled_height(0.0)
            .auto_shrink([false, false])
            .show(&mut child, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut self.agent.input)
                        .id(id)
                        .font(sans(13.0))
                        .frame(egui::Frame::NONE)
                        .margin(0.0)
                        .hint_text(hint)
                        .layouter(&mut layouter)
                        .return_key(egui::KeyboardShortcut::new(Modifiers::SHIFT, Key::Enter))
                        // while the `@` list shows, Tab picks from it and Esc closes it: neither
                        // takes the keys away from the box
                        .event_filter(egui::EventFilter {
                            tab: listing,
                            escape: listing,
                            horizontal_arrows: true,
                            vertical_arrows: true,
                        })
                        .desired_width(ui.available_width())
                        .desired_rows(1),
                )
            });
        let resp = scroll.inner;
        self.note("agent:input:viewport", scroll.inner_rect);
        self.note(
            "agent:input:content",
            Rect::from_min_size(scroll.inner_rect.min, scroll.content_size),
        );
        if resp.changed() {
            ui.ctx().request_repaint();
        }
        self.note("agent:input", text_r);
        if ui.input(|i| i.pointer.primary_clicked())
            && ui
                .input(|i| i.pointer.interact_pos())
                .is_some_and(|q| box_r.contains(q) && !text_r.contains(q))
        {
            resp.request_focus();
        }
        if send {
            match self.agent.waiting().map(str::to_string) {
                // an empty box: Enter answers the agent's question, as in a terminal
                // only a card on screen: one scrolled out of sight isn't answered unseen
                Some(id) if self.agent.input.trim().is_empty() && self.agent.card_visible => {
                    self.agent.answer(&id, Answer::Allow)
                }
                Some(_) if self.agent.input.trim().is_empty() => self.agent.jump = true,
                _ => self.agent.send(),
            }
        }
        // Shift+Tab in the box: the permission mode, as in a terminal
        if focused
            && self.agent.settings.kind == agent_settings::Kind::ClaudeCode
            && ui.input_mut(|i| i.consume_key(Modifiers::SHIFT, Key::Tab))
        {
            let m = if self.agent.mode_other.is_some() {
                Mode::Ask
            } else {
                self.agent.mode.next()
            };
            self.agent.set_mode(m);
            resp.request_focus();
        }

        // the row under the text: attach the selection, send or stop
        let row_y = box_r.bottom() - 26.0;
        let ar = Rect::from_min_size(pos2(box_r.left() + 4.0, row_y), vec2(24.0, 22.0));
        let sel = self.agent.selection.clone();
        let tip = match &sel {
            Some(s) => format!("Attach the selection: {s}"),
            None => "Select a feature, face or occurrence to attach it".into(),
        };
        let ar_resp = ui
            .interact(ar, ui.scope_id().with("agent-attach"), Sense::click())
            .on_hover_text(tip);
        ar_resp
            .widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, sel.is_some(), "Attach selection"));
        if ar_resp.hovered() && sel.is_some() {
            p.rect_filled(ar, 4.0, t.hover);
        }
        let ink = if sel.is_some() {
            t.ink
        } else {
            t.ink2.gamma_multiply(0.5)
        };
        p.text(ar.center(), Align2::CENTER_CENTER, "@", medium(14.0), ink);
        self.note("agent:attach", ar);
        if ar_resp.clicked()
            && let Some(s) = sel
        {
            // everything selected, when it's more than one
            let all = if self.agent.selection_all.contains(&s) {
                self.agent.selection_all.clone()
            } else {
                vec![s]
            };
            for s in all {
                self.agent.attach(&s);
            }
        }

        // markup: drawn on the window, sent with the message
        let mr = Rect::from_min_size(pos2(ar.right() + 2.0, row_y), vec2(24.0, 22.0));
        let marks = self.agent.markup.marks.len();
        let tip = match marks {
            0 => "Markup: draw on the window for the agent (K)".to_string(),
            n => format!(
                "Markup: {n} mark{} go with the message (K)",
                if n == 1 { "" } else { "s" }
            ),
        };
        let m_resp = ui
            .interact(mr, ui.scope_id().with("agent-markup"), Sense::click())
            .on_hover_text(tip);
        m_resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, "Markup"));
        if self.agent.markup.on {
            p.rect_filled(mr, 4.0, t.on);
        } else if m_resp.hovered() {
            p.rect_filled(mr, 4.0, t.hover);
        }
        icons::draw(&p, mr.center() - vec2(8.0, 8.0), 16.0, "markup", 1.5, t.ink);
        if marks > 0 {
            p.circle_filled(
                pos2(mr.right() - 3.0, mr.top() + 3.0),
                3.5,
                super::markup::INKS[0].1,
            );
        }
        self.note("agent:markup", mr);
        if m_resp.clicked() {
            self.agent.markup.toggle();
        }

        // the view as it is now, a picture with the message
        let vr = Rect::from_min_size(pos2(mr.right() + 2.0, row_y), vec2(24.0, 22.0));
        let on = self.agent.attach_view;
        let v_resp = ui
            .interact(vr, ui.scope_id().with("agent-view"), Sense::click())
            .on_hover_text(if on {
                "A picture of the view goes with the next message"
            } else {
                "Attach a picture of the view to the next message"
            });
        v_resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::CheckBox, true, on, "Attach view"));
        if on {
            p.rect_filled(vr, 4.0, t.on);
        } else if v_resp.hovered() {
            p.rect_filled(vr, 4.0, t.hover);
        }
        icons::draw(
            &p,
            vr.center() - vec2(8.0, 8.0),
            16.0,
            "camera",
            1.5,
            if on { t.on_ink } else { t.ink },
        );
        self.note("agent:view", vr);
        if v_resp.clicked() {
            self.agent.attach_view = !on;
        }

        let running = self.agent.running;
        let word = "Send";
        let sr = Rect::from_min_size(pos2(box_r.right() - 58.0, row_y), vec2(52.0, 22.0));
        let can = self.agent.trust_ask.is_none()
            && !self.agent.voice.busy()
            && !self.agent.markup.sending
            && (!self.agent.input.trim().is_empty()
                || marks > 0
                || self.agent.attach_view
                || !self.agent.pictures.is_empty());
        let s_resp = ui.interact(sr, ui.scope_id().with("agent-send"), Sense::click());
        s_resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, can, word));
        let fill = if can {
            t.focus.gamma_multiply(if s_resp.hovered() { 1.0 } else { 0.85 })
        } else {
            t.raised
        };
        p.rect_filled(sr, 4.0, fill);
        p.text(
            sr.center(),
            Align2::CENTER_CENTER,
            word,
            medium(12.0),
            if can { super::theme::ON_FOCUS } else { t.ink2 },
        );
        self.note("agent:send", sr);
        let mut mic_right = sr.left();
        if running {
            let stop = Rect::from_min_size(pos2(sr.left() - 58.0, row_y), vec2(52.0, 22.0));
            let response = ui.interact(stop, ui.scope_id().with("agent-stop"), Sense::click());
            response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, "Stop"));
            p.rect_filled(
                stop,
                4.0,
                t.error.gamma_multiply(if response.hovered() { 0.9 } else { 0.7 }),
            );
            p.text(
                stop.center(),
                Align2::CENTER_CENTER,
                "Stop",
                medium(12.0),
                Color32::WHITE,
            );
            self.note("agent:stop", stop);
            if response.clicked() {
                self.agent.out.push(AgentAct::Stop);
            }
            mic_right = stop.left();
        }
        let mic = Rect::from_min_size(pos2(mic_right - 30.0, row_y), vec2(24.0, 22.0));
        self.mic_button(ui, mic, true);
        if s_resp.clicked() && can {
            self.agent.send();
        }
    }
}

fn pulse(ui: &Ui) -> f32 {
    (ui.input(|i| i.time) as f32 * 5.0).sin() * 0.5 + 0.5
}

/// An attached id: `@id` in a pill, room permitting; its rect.
/// An attached id; `removable`: with its × (in the box, not in a sent message).
fn chip(ui: &mut Ui, t: &Theme, at: egui::Pos2, id: &str, room: f32, removable: bool) -> Rect {
    let g = ui.fonts_mut(|f| f.layout_no_wrap(format!("@{id}"), mono(11.0), t.ink));
    let w = (g.size().x + if removable { 26.0 } else { 14.0 }).min(room.max(40.0));
    let r = Rect::from_min_size(at, vec2(w, 20.0));
    let p = ui.painter().with_clip_rect(r);
    p.rect_filled(r, 10.0, t.on.gamma_multiply(0.6));
    p.galley(pos2(r.left() + 7.0, r.center().y - g.size().y * 0.5), g, t.ink);
    if removable {
        p.text(
            pos2(r.right() - 9.0, r.center().y),
            Align2::CENTER_CENTER,
            "×",
            sans(12.0),
            t.ink2,
        );
    }
    r
}

/// A pasted picture's 20 px thumbnail (PNG or JPEG; GIF and WebP have none), made once.
fn picture_thumb(ctx: &egui::Context, pic: &crate::agent::Picture) -> Option<egui::TextureHandle> {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    pic.bytes.hash(&mut h);
    let id = egui::Id::unique(("agent-picture-thumb", h.finish()));
    if let Some(t) = ctx.data(|d| d.get_temp::<Option<egui::TextureHandle>>(id)) {
        return t;
    }
    let made = crate::look::decode_image(&pic.bytes).map(|(w, h, rgba)| {
        // nearest pixels, at most 40 a side (20 points, twice for HiDPI)
        let s = (40.0 / w.max(h) as f32).min(1.0);
        let (tw, th) = (((w as f32 * s) as usize).max(1), ((h as f32 * s) as usize).max(1));
        let mut out = Vec::with_capacity(tw * th * 4);
        for y in 0..th {
            for x in 0..tw {
                let (sx, sy) = ((x as f32 / s) as usize, (y as f32 / s) as usize);
                let k = (sy.min(h as usize - 1) * w as usize + sx.min(w as usize - 1)) * 4;
                out.extend_from_slice(&rgba[k..k + 4]);
            }
        }
        let img = egui::ColorImage::from_rgba_unmultiplied([tw, th], &out);
        ctx.load_texture(format!("agent-picture-{id:?}"), img, egui::TextureOptions::LINEAR)
    });
    ctx.data_mut(|d| d.insert_temp(id, made.clone()));
    made
}

/// A pasted picture's chip: its thumbnail, its name, a ×.
fn picture_chip(
    ui: &mut Ui,
    t: &Theme,
    at: egui::Pos2,
    label: &str,
    thumb: Option<egui::TextureHandle>,
    room: f32,
) -> Rect {
    let g = ui.fonts_mut(|f| f.layout_no_wrap(label.to_string(), sans(11.0), t.ink));
    let lead = if thumb.is_some() { 20.0 } else { 0.0 };
    let w = (g.size().x + 26.0 + lead).min(room.max(40.0));
    let r = Rect::from_min_size(at, vec2(w, 20.0));
    let p = ui.painter().with_clip_rect(r);
    p.rect_filled(r, 10.0, t.raised);
    p.rect_stroke(r, 10.0, Stroke::new(1.0, t.line), StrokeKind::Inside);
    if let Some(tex) = &thumb {
        // as the picture is, in a 16 px square at the chip's left
        let size = tex.size_vec2();
        let s = 16.0 / size.x.max(size.y);
        let tr = Rect::from_center_size(pos2(r.left() + 12.0, r.center().y), size * s);
        p.image(
            tex.id(),
            tr,
            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            Color32::WHITE,
        );
    }
    // the name stops short of the ×
    ui.painter()
        .with_clip_rect(Rect::from_min_max(r.min, pos2(r.right() - 18.0, r.bottom())))
        .galley(
            pos2(r.left() + 7.0 + lead, r.center().y - g.size().y * 0.5),
            g,
            t.ink,
        );
    p.text(
        pos2(r.right() - 9.0, r.center().y),
        Align2::CENTER_CENTER,
        "×",
        sans(12.0),
        t.ink2,
    );
    r
}

/// An id in words, for a chip's hover: "End face of extrude1" for `extrude1.end`; the id as it
/// is otherwise.
pub fn describe(id: &str) -> String {
    // an edge as Forge names it, between two faces: "Edge between extrude1's side and start face"
    if let Some(inner) = id
        .strip_prefix("edges(between: (")
        .or_else(|| id.strip_prefix("edge(between: ("))
        .and_then(|r| r.strip_suffix("))"))
        && let Some((a, b)) = split_top(inner)
    {
        let face = |f: &str| -> Option<(String, String)> {
            // a face's own selector less its argument: extrude1.side(sk1.r1[0]) is a side
            let f = f.split_once('(').map_or(f, |x| x.0).trim();
            let (feat, role) = f.rsplit_once('.')?;
            let word = match role {
                "end" => "end face",
                "start" => "start face",
                "side" | "sides" => "side",
                "bore" => "bore",
                r => r,
            };
            Some((feat.to_string(), word.to_string()))
        };
        if let (Some((fa, ra)), Some((fb, rb))) = (face(a), face(b)) {
            return if fa == fb {
                format!("Edge between {fa}'s {ra} and {rb}")
            } else {
                format!("Edge between {fa}'s {ra} and {fb}'s {rb}")
            };
        }
    }
    let Some((feat, role)) = id.rsplit_once('.') else {
        return id.to_string();
    };
    let what = match role {
        "end" => "End face",
        "start" => "Start face",
        "side" | "sides" => "Side faces",
        "bore" => "Bore",
        "faces" => "Faces",
        "axis" => "Axis",
        _ => return id.to_string(),
    };
    format!("{what} of {feat}")
}

/// A selector's two arguments, split at the comma outside any brackets.
fn split_top(s: &str) -> Option<(&str, &str)> {
    let mut depth = 0i32;
    for (k, c) in s.char_indices() {
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => depth -= 1,
            ',' if depth == 0 => return Some((s[..k].trim(), s[k + 1..].trim())),
            _ => {}
        }
    }
    None
}

/// `describe` mid-sentence: "end face of extrude1"; an id it has no words for, as it is.
pub fn describe_lower(id: &str) -> String {
    let d = describe(id);
    if d == id {
        return d;
    }
    let mut c = d.chars();
    c.next()
        .map_or(d.clone(), |f| format!("{}{}", f.to_lowercase(), c.as_str()))
}

/// The selection offered for the next message: outlined, not filled, until it's sent.
fn offered_chip(ui: &mut Ui, t: &Theme, at: egui::Pos2, label: &str, room: f32) -> Rect {
    let g = ui.fonts_mut(|f| f.layout_no_wrap(label.to_string(), mono(11.0), t.ink2));
    let w = (g.size().x + 26.0).min(room.max(40.0));
    let r = Rect::from_min_size(at, vec2(w, 20.0));
    let p = ui.painter().with_clip_rect(r.expand(1.0));
    p.rect_stroke(
        r,
        10.0,
        Stroke::new(1.0, t.focus.gamma_multiply(0.7)),
        StrokeKind::Inside,
    );
    // the text stops short of the ×
    ui.painter()
        .with_clip_rect(Rect::from_min_max(r.min, pos2(r.right() - 18.0, r.bottom())))
        .galley(pos2(r.left() + 7.0, r.center().y - g.size().y * 0.5), g, t.ink2);
    p.text(
        pos2(r.right() - 9.0, r.center().y),
        Align2::CENTER_CENTER,
        "×",
        sans(12.0),
        t.ink2,
    );
    r
}

/// "Trust this folder for the agent?": the folder, Trust and Not now.
fn trust_card(ui: &mut Ui, t: &Theme, dir: &str, w: f32, places: &mut Vec<(String, Rect)>) {
    let inner = w - 2.0 * PAD - 16.0;
    let q = ui.fonts_mut(|f| {
        f.layout_job(plain_job(
            "Trust this folder for the agent?",
            medium(13.0),
            t.ink,
            inner,
        ))
    });
    let d = ui.fonts_mut(|f| f.layout_job(plain_job(dir, mono(11.0), t.ink2, inner)));
    let h = q.size().y + d.size().y + 52.0;
    let (r, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
    let card = Rect::from_min_max(pos2(PAD, r.top() + 4.0), pos2(w - PAD, r.bottom() - 4.0));
    let p = ui.painter();
    p.rect_filled(card, 6.0, t.raised);
    p.rect_stroke(card, 6.0, Stroke::new(1.0, t.focus), StrokeKind::Inside);
    p.galley(pos2(card.left() + 8.0, card.top() + 8.0), q.clone(), t.ink);
    p.galley(
        pos2(card.left() + 8.0, card.top() + 12.0 + q.size().y),
        d.clone(),
        t.ink2,
    );
    let by = card.bottom() - 28.0;
    let yes = Rect::from_min_size(pos2(card.left() + 8.0, by), vec2(60.0, 22.0));
    let no = Rect::from_min_size(pos2(yes.right() + 6.0, by), vec2(70.0, 22.0));
    p.rect_filled(yes, 4.0, t.focus.gamma_multiply(0.85));
    p.text(
        yes.center(),
        Align2::CENTER_CENTER,
        "Trust",
        medium(12.0),
        super::theme::ON_FOCUS,
    );
    p.rect_filled(no, 4.0, t.field);
    p.rect_stroke(no, 4.0, Stroke::new(1.0, t.line), StrokeKind::Inside);
    p.text(no.center(), Align2::CENTER_CENTER, "Not now", sans(12.0), t.ink);
    places.push(("agent:trust".into(), card));
    places.push(("agent:trust:yes".into(), yes));
    places.push(("agent:trust:no".into(), no));
}

/// A permission's card, as drawn.
struct Card<'a> {
    id: &'a str,
    summary: &'a str,
    input: &'a str,
    always: Option<&'a str>,
    state: Asked,
    /// The one Enter and Esc answer.
    first: bool,
    /// What the change is, in words, when it's one param's value: "extrude1 depth from 12 to 14 mm".
    headline: Option<String>,
}

/// A small down chevron centred at `c`.
fn down_chevron(p: &egui::Painter, c: egui::Pos2, ink: Color32) {
    let s = Stroke::new(1.6, ink);
    p.line_segment([c + vec2(-4.0, -2.0), c + vec2(0.0, 2.0)], s);
    p.line_segment([c + vec2(0.0, 2.0), c + vec2(4.0, -2.0)], s);
}

/// A time as a person reads it: "8 s", "2 min 5 s".
/// The plan's limit in words: reached or close, and when it resets ("in 3 h 20 min").
pub fn limit_words(reached: bool, resets: Option<f64>, now: f64) -> String {
    let what = if reached {
        "Plan limit reached"
    } else {
        "Close to your plan's limit"
    };
    let Some(at) = resets else { return what.into() };
    let mins = ((at - now) / 60.0).ceil().max(0.0) as u64;
    let when = match mins {
        0 => return format!("{what} · resets now"),
        1..60 => format!("{mins} min"),
        60..1440 if mins.is_multiple_of(60) => format!("{} h", mins / 60),
        60..1440 => format!("{} h {} min", mins / 60, mins % 60),
        _ => format!("{} days", mins.div_ceil(1440)),
    };
    format!("{what} · resets in {when}")
}

pub fn duration(secs: f64) -> String {
    let s = secs.max(0.0).round() as u64;
    if s < 60 {
        format!("{s} s")
    } else {
        format!("{} min {} s", s / 60, s % 60)
    }
}

/// A count, short: "340", "21.3k", "1.2M".
pub fn thousands(n: u64) -> String {
    match n {
        0..1000 => n.to_string(),
        1000..1_000_000 => format!("{:.1}k", n as f64 / 1000.0),
        _ => format!("{:.1}M", n as f64 / 1e6),
    }
}

/// The diff's colours in this theme: added ink and fill, removed ink and fill.
fn diff_inks(t: &Theme) -> (Color32, Color32, Color32, Color32) {
    if t.dark {
        (ADD_INK, ADD_BG, DEL_INK, DEL_BG)
    } else {
        (
            Color32::from_rgb(0x1A, 0x6B, 0x35),
            Color32::from_rgb(0xDC, 0xF2, 0xE2),
            Color32::from_rgb(0xA8, 0x24, 0x24),
            Color32::from_rgb(0xFA, 0xE0, 0xE0),
        )
    }
}

/// A tool's input as a terminal shows a diff: its `- ` lines red on a red tint, its `+ ` lines
/// green on a green tint, the rest as it is.
fn diff_job(text: &str, t: &Theme, width: f32) -> LayoutJob {
    let (add_ink, add_bg, del_ink, del_bg) = diff_inks(t);
    let mut job = LayoutJob::default();
    job.wrap.max_width = width;
    let lines: Vec<&str> = text.split('\n').collect();
    for (k, line) in lines.iter().enumerate() {
        let (color, background) = if line.starts_with("- ") {
            (del_ink, del_bg)
        } else if line.starts_with("+ ") {
            (add_ink, add_bg)
        } else {
            (t.ink2, Color32::TRANSPARENT)
        };
        let nl = if k + 1 < lines.len() { "\n" } else { "" };
        job.append(
            &format!("{line}{nl}"),
            0.0,
            TextFormat {
                font_id: mono(11.0),
                color,
                background,
                ..Default::default()
            },
        );
    }
    job
}

/// An edit's lines as a card shows them: `- ` taken out, `+ ` put in, `  ` around them, `…` for
/// the unchanged runs left out.
pub fn diff_text(lines: &[Line]) -> String {
    lines
        .iter()
        .map(|l| match l {
            Line::Same(s) => format!("  {s}"),
            Line::Add(s) => format!("+ {s}"),
            Line::Del(s) => format!("- {s}"),
            Line::Skip(_) => "  …".to_string(),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// What an edit tool would make of a file's text: Edit's replacement (every one with
/// `replace_all`), MultiEdit's in turn, Write's whole. None when it isn't one of those or its
/// text isn't in the file (Claude Code would refuse it too).
pub fn edited(tool: &str, input: &forge_agent::json::Json, now: &str) -> Option<String> {
    use forge_agent::json::Json;
    let one = |text: &str, e: &Json| -> Option<String> {
        let old = e.get("old_string").and_then(Json::str)?;
        let new = e.get("new_string").and_then(Json::str)?;
        if old.is_empty() || !text.contains(old) {
            return None;
        }
        Some(if e.get("replace_all") == Some(&Json::Bool(true)) {
            text.replace(old, new)
        } else {
            text.replacen(old, new, 1)
        })
    };
    match tool {
        "Edit" => one(now, input),
        "MultiEdit" => match input.get("edits") {
            Some(Json::Arr(es)) => es.iter().try_fold(now.to_string(), |t, e| one(&t, e)),
            _ => None,
        },
        "Write" => input.get("content").and_then(Json::str).map(str::to_string),
        _ => None,
    }
}

/// How long ago `updated` (seconds since the epoch) was, in words: "just now", "5 min ago".
pub fn ago(updated: u64) -> String {
    let age = (forge_history::now() as u64).saturating_sub(updated);
    if age < 60 {
        "just now".into()
    } else if age < 3600 {
        format!("{} min ago", age / 60)
    } else if age < 86400 {
        format!("{} h ago", age / 3600)
    } else {
        format!("{} days ago", age / 86400)
    }
}

/// The line between a resumed chat's old part and what's said since: when it was from, and a
/// way to start afresh instead.
fn resumed_divider(ui: &mut Ui, t: &Theme, w: f32, said: &str, places: &mut Vec<(String, Rect)>) -> bool {
    let g = ui.fonts_mut(|f| f.layout_no_wrap(said.to_string(), sans(11.0), t.ink2));
    let new = ui.fonts_mut(|f| f.layout_no_wrap("New chat".to_string(), medium(11.0), t.focus));
    let (r, _) = ui.allocate_exact_size(vec2(w, 26.0), Sense::hover());
    let y = r.center().y;
    let text_l = PAD + 18.0;
    let p = ui.painter();
    p.line_segment([pos2(PAD, y), pos2(text_l - 6.0, y)], Stroke::new(1.0, t.line));
    p.galley(pos2(text_l, y - g.size().y * 0.5), g.clone(), t.ink2);
    let nr = Rect::from_min_size(
        pos2(w - PAD - new.size().x - 4.0, y - 9.0),
        vec2(new.size().x + 4.0, 18.0),
    );
    p.line_segment(
        [pos2(text_l + g.size().x + 6.0, y), pos2(nr.left() - 6.0, y)],
        Stroke::new(1.0, t.line),
    );
    let hit = ui
        .interact(nr, ui.scope_id().with("agent-resumed-new"), Sense::click())
        .on_hover_text("Start a new chat; this one stays in Chats");
    hit.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, "New chat"));
    if hit.hovered() {
        ui.painter().rect_filled(nr, 4.0, t.hover);
    }
    ui.painter()
        .galley(pos2(nr.left() + 2.0, y - new.size().y * 0.5), new, t.focus);
    places.push(("agent:resumed".into(), r));
    places.push(("agent:resumed:new".into(), nr));
    hit.clicked()
}

/// A message queued behind the turn: "Queued · its words", Edit, ×.
fn queued_row(
    ui: &mut Ui,
    t: &Theme,
    w: f32,
    q: usize,
    text: &str,
    places: &mut Vec<(String, Rect)>,
) -> Option<AgentAct> {
    let (r, _) = ui.allocate_exact_size(vec2(w, 24.0), Sense::hover());
    let row = Rect::from_min_max(pos2(PAD, r.top() + 1.0), pos2(w - PAD, r.bottom() - 1.0));
    let p = ui.painter();
    p.rect_stroke(row, 4.0, Stroke::new(1.0, t.line), StrokeKind::Inside);
    let x_r = Rect::from_min_size(
        pos2(row.right() - 22.0, row.top() + 1.0),
        vec2(20.0, row.height() - 2.0),
    );
    let edit = ui.fonts_mut(|f| f.layout_no_wrap("Edit".into(), medium(11.0), t.focus));
    let e_r = Rect::from_min_size(
        pos2(x_r.left() - edit.size().x - 10.0, row.top() + 1.0),
        vec2(edit.size().x + 8.0, row.height() - 2.0),
    );
    let mut job = plain_job(
        &format!("Queued · {}", one_line(text, 200)),
        sans(11.0),
        t.ink2,
        e_r.left() - row.left() - 12.0,
    );
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    job.wrap.overflow_character = Some('…');
    let g = ui.fonts_mut(|f| f.layout_job(job));
    p.galley(
        pos2(row.left() + 8.0, row.center().y - g.size().y * 0.5),
        g,
        t.ink2,
    );
    let e = ui
        .interact(e_r, ui.scope_id().with(("agent-queued-edit", q)), Sense::click())
        .on_hover_text("Take it back into the box to change it; it's sent when you send it again");
    e.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, "Edit queued message"));
    let x = ui
        .interact(x_r, ui.scope_id().with(("agent-queued-drop", q)), Sense::click())
        .on_hover_text("Don't send it");
    x.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, "Drop queued message"));
    let p = ui.painter();
    for (resp, rr) in [(&e, e_r), (&x, x_r)] {
        if resp.hovered() {
            p.rect_filled(rr, 3.0, t.hover);
        }
    }
    p.galley(
        pos2(e_r.left() + 4.0, e_r.center().y - edit.size().y * 0.5),
        edit,
        t.focus,
    );
    p.text(x_r.center(), Align2::CENTER_CENTER, "×", sans(13.0), t.ink2);
    places.push((format!("agent:queued:{q}"), row));
    places.push((format!("agent:queued:{q}:edit"), e_r));
    places.push((format!("agent:queued:{q}:drop"), x_r));
    if e.clicked() {
        Some(AgentAct::Unqueue { index: q, edit: true })
    } else if x.clicked() {
        Some(AgentAct::Unqueue {
            index: q,
            edit: false,
        })
    } else {
        None
    }
}

/// How far beyond the chat's visible part entries are still drawn, in points.
const VIRTUAL_MARGIN: f32 = 600.0;

/// An entry's height as drawn, kept by its index.
fn set_height(heights: &mut Vec<f32>, k: usize, h: f32) {
    if heights.len() <= k {
        heights.resize(k + 1, 0.0);
    }
    heights[k] = h;
}

/// Pictures sent with a message, said on it: "1 picture", "3 pictures".
pub fn pictures_word(n: usize) -> String {
    format!("{n} picture{}", if n == 1 { "" } else { "s" })
}

/// What the empty chat says after `/new`.
pub const STARTED_NEW: &str = "Started a new chat · the last one is in Chats";

/// What starts the page's agent: the command, to copy.
pub const BRIDGE_COMMAND: &str = "forgecad web . --agent";

/// The Agent tab in a page with no bridge (§7's empty state): what the agent is here, the command
/// that starts it (mono, with Copy), and the desktop app as the other way.
fn no_bridge(ui: &mut Ui, t: &Theme, places: &mut Vec<(String, Rect)>) {
    use super::theme;
    ui.vertical_centered(|ui| {
        ui.add_space(theme::S3);
        ui.label(
            egui::RichText::new("Use your own Claude Code here")
                .font(theme::title())
                .color(t.ink),
        );
        ui.label(
            egui::RichText::new(
                "In the browser, the agent is Claude Code on your computer, with your own plan and \
                 settings. Run this in the project folder there, then open the link it prints.",
            )
            .font(theme::small())
            .color(t.ink2),
        );
        ui.add_space(theme::S2);
        let cmd = ui.label(egui::RichText::new(BRIDGE_COMMAND).font(mono(12.0)).color(t.ink));
        places.push(("agent:bridge:command".into(), cmd.rect));
        ui.add_space(theme::S1);
        let copy = ui.button("Copy command");
        places.push(("agent:bridge:copy".into(), copy.rect));
        if copy.clicked() {
            ui.ctx().copy_text(BRIDGE_COMMAND.into());
        }
        ui.add_space(theme::S2);
        ui.label(
            egui::RichText::new("Or open the project in the ForgeCAD desktop app.")
                .font(theme::small())
                .color(t.ink2),
        );
    });
}

/// One param's change in an edit's -/+ lines (`param KEY = OLD` → `param KEY = NEW`): the key,
/// what it was and what it becomes.
pub fn param_change(input: &str) -> Option<(String, String, String)> {
    let mut old = None;
    let mut new = None;
    for l in input.lines() {
        let (sign, rest) = if let Some(r) = l.strip_prefix("- ") {
            ('-', r)
        } else if let Some(r) = l.strip_prefix("+ ") {
            ('+', r)
        } else {
            continue;
        };
        let (key, value) = rest.trim().strip_prefix("param ")?.split_once('=')?;
        let slot = if sign == '-' { &mut old } else { &mut new };
        if slot.is_some() {
            return None;
        }
        *slot = Some((key.trim().to_string(), value.trim().to_string()));
    }
    let ((k0, a), (k1, b)) = (old?, new?);
    (k0 == k1 && a != b).then_some((k0, a, b))
}

/// Lines of a tool's input shown on its card; the rest is one "… more" line.
const CARD_LINES: usize = 14;

/// The agent asking before a tool runs: what it would do, and the answers, full width (the panel
/// is narrow). Answered, it folds to one line saying what was said.
fn permission_card(ui: &mut Ui, t: &Theme, c: Card, w: f32, places: &mut Vec<(String, Rect)>) {
    let inner = w - 2.0 * PAD - 16.0;
    if c.state != Asked::Waiting {
        let (word, ink) = match c.state {
            Asked::Allowed => ("Allowed", t.success),
            Asked::Always => ("Allowed from now on", t.success),
            Asked::Denied => ("Declined", t.error),
            _ => ("Not answered", t.ink2),
        };
        let g = ui.fonts_mut(|f| {
            f.layout_job(plain_job(
                &format!("{word}: {}", c.summary),
                sans(12.0),
                t.ink2,
                inner,
            ))
        });
        let (r, _) = ui.allocate_exact_size(vec2(w, g.size().y + 8.0), Sense::hover());
        ui.painter().circle_filled(
            pos2(PAD + 4.0, r.top() + 4.0 + g.size().y.min(16.0) * 0.5),
            3.0,
            ink,
        );
        selectable(ui, pos2(PAD + 12.0, r.top() + 4.0), g);
        // `put` leaves the cursor under the text: back under the row
        ui.advance_cursor_after_rect(r);
        places.push((format!("agent:perm:{}", c.id), r));
        return;
    }
    let head = ui.fonts_mut(|f| f.layout_job(plain_job("Permission needed", medium(11.0), t.warn, inner)));
    let what = ui.fonts_mut(|f| f.layout_job(plain_job(c.summary, medium(13.0), t.ink, inner)));
    let lines: Vec<&str> = c.input.trim_end().lines().collect();
    let mut body = lines
        .iter()
        .take(CARD_LINES)
        .copied()
        .collect::<Vec<_>>()
        .join("\n");
    if lines.len() > CARD_LINES {
        body.push_str(&format!("\n… {} more lines", lines.len() - CARD_LINES));
    }
    let code = (!body.trim().is_empty() && body.trim() != c.summary.trim())
        .then(|| ui.fonts_mut(|f| f.layout_job(diff_job(&body, t, inner - 12.0))));
    let code_h = code.as_ref().map_or(0.0, |g| g.size().y + 14.0);
    let headline = c
        .headline
        .as_ref()
        .map(|h| ui.fonts_mut(|f| f.layout_job(plain_job(h, medium(12.0), t.ink, inner))));
    let head_h = headline.as_ref().map_or(0.0, |g| g.size().y + 6.0);
    const BTN: f32 = 26.0;
    let buttons = if c.always.is_some() { 3.0 } else { 2.0 };
    let h = 10.0 + head.size().y + 2.0 + what.size().y + 8.0 + head_h + code_h + buttons * (BTN + 6.0) + 4.0;
    ui.add_space(4.0);
    let (r, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
    let card = Rect::from_min_max(pos2(PAD, r.top()), pos2(w - PAD, r.bottom()));
    let p = ui.painter().clone();
    p.rect_filled(card, 6.0, t.raised);
    p.rect_stroke(card, 6.0, Stroke::new(1.5, t.warn), StrokeKind::Inside);
    let x = card.left() + 8.0;
    let mut y = card.top() + 10.0;
    p.galley(pos2(x, y), head.clone(), t.warn);
    y += head.size().y + 2.0;
    selectable(ui, pos2(x, y), what.clone());
    y += what.size().y + 8.0;
    if let Some(g) = headline {
        let gh = g.size().y;
        selectable(ui, pos2(x, y), g);
        y += gh + 6.0;
    }
    if let Some(g) = code {
        let box_r = Rect::from_min_max(pos2(x, y), pos2(card.right() - 8.0, y + g.size().y + 8.0));
        p.rect_filled(box_r, 4.0, t.field);
        selectable(ui, pos2(x + 6.0, y + 4.0), g);
        y = box_r.bottom() + 6.0;
    }
    let mut button = |name: &str, label: String, key: &str, primary: bool, tip: String| {
        let b = Rect::from_min_max(pos2(x, y), pos2(card.right() - 8.0, y + BTN));
        let resp = ui.interact(
            b,
            ui.scope_id().with(("agent-perm-hover", c.id, name)),
            Sense::hover(),
        );
        resp.clone().on_hover_text(tip);
        resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &label));
        if primary {
            p.rect_filled(
                b,
                4.0,
                if resp.hovered() {
                    t.focus
                } else {
                    t.focus.gamma_multiply(0.85)
                },
            );
        } else {
            p.rect_filled(b, 4.0, if resp.hovered() { t.hover } else { t.field });
            p.rect_stroke(b, 4.0, Stroke::new(1.0, t.line), StrokeKind::Inside);
        }
        // dark ink on the amber: white on it is under 3:1 in either theme
        let ink = if primary { super::theme::ON_FOCUS } else { t.ink };
        let room = if key.is_empty() {
            b.width() - 16.0
        } else {
            b.width() - 56.0
        };
        let g = ui.fonts_mut(|f| f.layout_no_wrap(label.clone(), medium(12.0), ink));
        p.with_clip_rect(Rect::from_min_max(b.min, pos2(b.left() + 8.0 + room, b.bottom())))
            .galley(pos2(b.left() + 8.0, b.center().y - g.size().y * 0.5), g, ink);
        if !key.is_empty() && c.first {
            p.text(
                pos2(b.right() - 8.0, b.center().y),
                Align2::RIGHT_CENTER,
                key,
                sans(11.0),
                if primary {
                    super::theme::ON_FOCUS.gamma_multiply(0.75)
                } else {
                    t.ink2
                },
            );
        }
        places.push((format!("agent:perm:{}:{name}", c.id), b));
        y += BTN + 6.0;
    };
    button("allow", "Allow".into(), "Enter", true, "Run it this once".into());
    if let Some(a) = c.always {
        button(
            "always",
            format!("Always: {a}"),
            "",
            false,
            format!("Run it, and {a}"),
        );
    }
    button(
        "deny",
        "Deny".into(),
        "Esc",
        false,
        "Don't run it; the agent is told and carries on".into(),
    );
    // `put` (the selectable text) leaves the cursor under the last text: back under the card
    ui.advance_cursor_after_rect(r);
    places.push((format!("agent:perm:{}", c.id), card));
}

fn problem_card(ui: &mut Ui, t: &Theme, pr: &Problem, w: f32, places: &mut Vec<(String, Rect)>) {
    let inner = w - 2.0 * PAD - 16.0;
    let g = ui.fonts_mut(|f| f.layout_job(plain_job(&pr.text, medium(13.0), t.ink, inner)));
    let fix_h = if pr.fix.is_some() { 52.0 } else { 0.0 };
    let (r, _) = ui.allocate_exact_size(vec2(w, g.size().y + 18.0 + fix_h), Sense::hover());
    let card = Rect::from_min_max(pos2(PAD, r.top()), pos2(w - PAD, r.bottom()));
    let p = ui.painter();
    p.rect_filled(card, 6.0, t.raised);
    p.rect_stroke(
        card,
        6.0,
        Stroke::new(1.0, t.error.gamma_multiply(0.7)),
        StrokeKind::Inside,
    );
    p.galley(pos2(card.left() + 8.0, card.top() + 8.0), g.clone(), t.ink);
    places.push(("agent:problem".into(), card));
    if let Some(fix) = &pr.fix {
        let y = card.top() + 12.0 + g.size().y;
        p.text(
            pos2(card.left() + 8.0, y + 6.0),
            Align2::LEFT_CENTER,
            "Run in a terminal:",
            sans(12.0),
            t.ink2,
        );
        let cmd = Rect::from_min_max(
            pos2(card.left() + 8.0, y + 16.0),
            pos2(card.right() - 8.0, y + 40.0),
        );
        p.rect_filled(cmd, 4.0, t.field);
        p.with_clip_rect(cmd.shrink(2.0)).text(
            pos2(cmd.left() + 6.0, cmd.center().y),
            Align2::LEFT_CENTER,
            fix,
            mono(12.0),
            t.ink,
        );
        let copy = Rect::from_min_size(
            pos2(cmd.right() - 46.0, cmd.top() + 2.0),
            vec2(44.0, cmd.height() - 4.0),
        );
        p.rect_filled(copy, 3.0, t.raised);
        p.text(copy.center(), Align2::CENTER_CENTER, "Copy", sans(11.0), t.ink);
        places.push(("agent:copy-fix".into(), copy));
    }
    ui.add_space(8.0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_lines_read_as_markdown() {
        let ink = Color32::WHITE;
        let code = Color32::RED;
        let runs = |l: &str| md_line(l, ink, code);
        let words = |l: &str| runs(l).iter().map(|r| r.0.clone()).collect::<String>();
        // a bullet's mark, a heading's hashes, a quote's mark: gone; what's left is the text
        assert_eq!(words("- one `two` **three**"), "• one two three");
        assert_eq!(words("  * nested"), "  • nested");
        assert_eq!(words("## Changes"), "Changes");
        assert_eq!(words("> said"), "said");
        // bold in the bold face, code in mono and its ink, a heading in the bold face
        let r = runs("a **b** `c`");
        assert_eq!(r.iter().find(|x| x.0 == "b").unwrap().1.font_id, semi(13.0));
        let c = r.iter().find(|x| x.0 == "c").unwrap();
        assert_eq!((c.1.font_id.clone(), c.1.color), (mono(12.0), code));
        assert_eq!(runs("# Title")[0].1.font_id, semi(14.0));
        // a table's row whole, in mono: its columns line up
        let t = runs("| a | b |");
        assert_eq!((t.len(), t[0].1.font_id.clone()), (1, mono(12.0)));
    }

    #[test]
    fn an_edge_between_two_faces_is_said_in_words() {
        assert_eq!(
            describe("edges(between: (extrude1.side(sk1.r1[0]), extrude1.start))"),
            "Edge between extrude1's side and start face"
        );
        assert_eq!(
            describe_lower("edge(between: (extrude1.end, fillet1.faces))"),
            "edge between extrude1's end face and fillet1's faces"
        );
        // what it has no words for stays as it is
        assert_eq!(describe("edges(of: extrude1)"), "edges(of: extrude1)");
    }

    #[test]
    fn a_table_is_cut_out_of_its_prose_and_read_as_cells() {
        let text = "Here:\n| part | d |\n|---|--:|\n| plate | `14` |\nDone.";
        assert_eq!(
            tables(text),
            vec![
                (false, "Here:"),
                (true, "| part | d |\n|---|--:|\n| plate | `14` |"),
                (false, "Done.")
            ]
        );
        assert_eq!(
            table_cells("| part | d |\n|---|--:|\n| plate | `14` |"),
            vec![vec!["part", "d"], vec!["plate", "`14`"]]
        );
        assert_eq!(tables("plain"), vec![(false, "plain")]);
    }

    #[test]
    fn a_reply_is_cut_into_prose_and_its_code_blocks() {
        let s = segments("Set it.\n```fg\nparam d = 14\nparam e = 2\n```\nDone.\n```\nhalf");
        assert_eq!(
            s,
            vec![
                (false, "Set it."),
                (true, "param d = 14\nparam e = 2"),
                (false, "Done."),
                (true, "half"),
            ]
        );
        assert_eq!(segments("plain"), vec![(false, "plain")]);
    }

    #[test]
    fn diff_cuts_unchanged_runs() {
        let old = "a\nb\nc\nd\ne\nf\ng\nh\n";
        let new = "a\nb\nc\nd\nE\nf\ng\nh\n";
        let d = diff(old, new, 1);
        assert_eq!(
            d,
            vec![
                Line::Skip(3),
                Line::Same("d".into()),
                Line::Del("e".into()),
                Line::Add("E".into()),
                Line::Same("f".into()),
                Line::Skip(2),
            ]
        );
        assert_eq!(counts(&d), (1, 1));
        assert!(diff(old, old, 2).is_empty());
        assert_eq!(diff("", "x\n", 2), vec![Line::Add("x".into())]);
    }

    #[test]
    fn transcript_takes_a_turn() {
        let mut a = AgentTab {
            input: "make it thicker\n".into(),
            ..Default::default()
        };
        a.attach("extrude1.end");
        a.send();
        assert!(a.running);
        assert_eq!(
            a.out,
            vec![AgentAct::Send {
                text: "make it thicker".into(),
                refs: vec!["extrude1.end".into()],
                markup: false,
                pictures: vec![],
                shown: true,
            }]
        );
        a.take(Event::Text("I'll ".into()));
        a.take(Event::Text("edit it.".into()));
        a.take(Event::ToolStart {
            id: "t1".into(),
            name: "Edit".into(),
        });
        a.take(Event::ToolInput {
            id: "t1".into(),
            summary: "Edited p.fg".into(),
            input: "- a\n+ b".into(),
        });
        a.take(Event::ToolDone {
            id: "t1".into(),
            output: "ok".into(),
            error: false,
        });
        a.take(Event::Text("Done.".into()));
        a.take(Event::Done {
            error: None,
            stopped: false,
        });
        assert!(!a.running);
        assert_eq!(a.entries.len(), 4);
        assert_eq!(
            a.entries[1],
            Entry::Agent {
                text: "I'll edit it.".into()
            }
        );
        assert!(
            matches!(&a.entries[2], Entry::Tool { summary, state: ToolState::Done, .. } if summary == "Edited p.fg")
        );
        assert_eq!(a.entries[3], Entry::Agent { text: "Done.".into() });
        assert_eq!(prompt("x", &["a.b".into()]), "x\n\n(Selected in the app: `a.b`)");
        assert_eq!(
            undo_label("\n make the base plate 10 mm thicker\nplease"),
            "Agent: make the base plate 10 mm thicker"
        );
    }
}

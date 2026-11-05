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
}

/// What the tab asks of the host.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentAct {
    Send {
        text: String,
        refs: Vec<String>,
    },
    Stop,
    NewChat,
    /// A turn's change taken back, or put back.
    Undo(usize, usize),
    Redo(usize, usize),
    Settings(SettingsAct),
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
    pub problem: Option<Problem>,
    /// The open file's name, for the context line.
    pub file: Option<String>,
    pub settings: AgentSettings,
    pub switcher: bool,
    pub out: Vec<AgentAct>,
    /// Tool rows opened to show what they ran.
    pub open: std::collections::HashSet<String>,
    /// What the selection is, as the agent names it, if anything's selected.
    pub selection: Option<String>,
    /// A folder outside the projects tree, asked about once; the message waiting on it.
    pub trust_ask: Option<String>,
    pub pending: Option<(String, Vec<String>)>,
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
            Event::Text(s) => match self.entries.last_mut() {
                Some(Entry::Agent { text }) => text.push_str(&s),
                _ => {
                    let s = s.trim_start_matches('\n').to_string();
                    if !s.is_empty() {
                        self.entries.push(Entry::Agent { text: s });
                    }
                }
            },
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
                self.running = false;
                for e in &mut self.entries {
                    if let Entry::Tool { state, .. } = e
                        && *state == ToolState::Running
                    {
                        *state = ToolState::Failed;
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
            Event::FileChanged { .. } => {}
        }
    }

    fn tool(&mut self, want: &str) -> Option<&mut Entry> {
        self.entries
            .iter_mut()
            .rev()
            .find(|e| matches!(e, Entry::Tool { id, .. } if id == want))
    }

    /// The input as a message: sent, if there's anything to send and nothing running.
    pub fn send(&mut self) {
        let text = self.input.trim().to_string();
        if text.is_empty() || self.running {
            return;
        }
        let refs = std::mem::take(&mut self.refs);
        self.entries.push(Entry::User {
            text: text.clone(),
            refs: refs.clone(),
        });
        self.input.clear();
        self.problem = None;
        self.running = true;
        self.out.push(AgentAct::Send { text, refs });
    }

    pub fn new_chat(&mut self) {
        self.entries.clear();
        self.refs.clear();
        self.running = false;
        self.problem = None;
        self.open.clear();
        self.out.push(AgentAct::NewChat);
    }

    pub fn attach(&mut self, id: &str) {
        if !id.is_empty() && !self.refs.iter().any(|r| r == id) {
            self.refs.push(id.to_string());
        }
    }

    /// Esc stops a running agent when the tab's showing or its input has the keys.
    pub fn takes_escape(&mut self, ctx: &egui::Context) -> bool {
        let focused = ctx.memory(|m| m.has_focus(egui::Id::unique(INPUT_ID)));
        if self.tab != SideTab::Agent || !(self.running || focused) {
            return false;
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
        "undo_view" => ("Took back its last view step".into(), whole()),
        "fg_run" | "fg_check" => (format!("Built and checked {}", s(input, "file")), whole()),
        "fg_edit" => (format!("Applied an edit to {}", s(input, "file")), whole()),
        "fg_handles" => (format!("Listed the parameters of {}", s(input, "file")), whole()),
        "fg_reference" => ("Read the Forge reference".into(), whole()),
        _ => (name.to_string(), whole()),
    }
}

/// The reply's text laid out: `code` and fenced blocks in mono, ** dropped.
fn rich(text: &str, width: f32, ink: Color32, code_ink: Color32) -> LayoutJob {
    let mut job = LayoutJob::default();
    job.wrap.max_width = width;
    let plain = TextFormat {
        font_id: sans(13.0),
        color: ink,
        ..Default::default()
    };
    let code = TextFormat {
        font_id: mono(12.0),
        color: code_ink,
        ..Default::default()
    };
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
        let line = line.replace("**", "");
        for (n, part) in line.split('`').enumerate() {
            if !part.is_empty() {
                job.append(part, 0.0, if n % 2 == 1 { code.clone() } else { plain.clone() });
            }
        }
    }
    job
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
        for (tab, word) in [(SideTab::Model, "Model"), (SideTab::Agent, "Agent")] {
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
                self.agent.tab = tab;
            }
            if on {
                p.rect_filled(r, 3.0, t.surface);
                p.rect_filled(
                    Rect::from_min_size(pos2(r.left(), r.bottom() - 2.0), vec2(tw, 2.0)),
                    0.0,
                    t.focus,
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
            if tab == SideTab::Agent && !on && self.agent.running {
                p.circle_filled(pos2(r.right() - 6.0, r.top() + 7.0), 3.0, t.focus);
            }
            self.note(format!("side:tab:{word}"), r);
            x += tw + 2.0;
        }
        if self.agent.tab == SideTab::Model {
            let room = w - x - 16.0;
            let g = ui.fonts_mut(|f| f.layout_no_wrap(right.to_string(), sans(12.0), t.ink2));
            if g.size().x <= room {
                p.galley(
                    pos2(w - 10.0 - g.size().x, y + (HEAD_H - g.size().y) * 0.5),
                    g,
                    t.ink2,
                );
            }
        } else {
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
        let t = self.theme;
        self.agent.selection = self.agent_selection();
        let drawing = self.drawing.as_ref().map(|d| d.name());
        self.agent.file = drawing
            .or_else(|| self.fg.as_ref().map(|f| base(&f.file).to_string()))
            .filter(|f| !f.is_empty())
            .or_else(|| (!self.name.is_empty()).then(|| self.name.clone()));
        let p = ui.painter().clone();
        let w = area.width();

        // the backend and model, a click to switch
        let bar = Rect::from_min_size(area.min, vec2(w, BAR_H));
        let label = self.agent.settings.label();
        let g = ui.fonts_mut(|f| f.layout_no_wrap(label.clone(), medium(12.0), t.ink));
        let bw = (g.size().x + 30.0).min(w - 2.0 * PAD);
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

        // what it's working on
        let ctx_y = bar.bottom();
        let on = match &self.agent.file {
            Some(f) => f.clone(),
            None => "no file open".into(),
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

        // the input's height follows its lines
        let lines = self.agent.input.lines().count().max(1) + usize::from(self.agent.input.ends_with('\n'));
        let chips = if self.agent.refs.is_empty() { 0.0 } else { 26.0 };
        let input_h = (lines.min(8) as f32 * 17.0 + 44.0 + chips).max(78.0 + chips);
        let input = Rect::from_min_max(pos2(area.left(), area.bottom() - input_h), area.max);
        let chat = Rect::from_min_max(pos2(area.left(), top), pos2(area.right(), input.top()));
        self.agent_chat(ui, chat);
        self.agent_input(ui, input);
        if self.agent.running {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(120));
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
        let code_ink = if t.dark {
            Color32::from_rgb(0xC9, 0xD7, 0xF0)
        } else {
            Color32::from_rgb(0x24, 0x3B, 0x63)
        };
        let a = &self.agent;
        egui::ScrollArea::vertical()
            .id_salt("agent-chat")
            .auto_shrink([false, false])
            .stick_to_bottom(true)
            .show(&mut child, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                ui.add_space(8.0);
                let mut tools_seen = 0;
                if let Some(pr) = &a.problem {
                    problem_card(ui, &t, pr, w, &mut places);
                }
                if a.entries.is_empty() && a.problem.is_none() {
                    let what = match &a.file {
                        Some(f) if self.fg.is_some() || self.drawing.is_some() => {
                            format!("Ask for a change to {f}")
                        }
                        _ => "Open a Forge part to work on it with the agent".into(),
                    };
                    let g = ui.fonts_mut(|f| f.layout_job(plain_job(&what, sans(13.0), t.ink2, inner)));
                    let (r, _) = ui.allocate_exact_size(vec2(w, g.size().y + 16.0), Sense::hover());
                    ui.painter().galley(pos2(PAD, r.top() + 8.0), g, t.ink2);
                }
                for (k, e) in a.entries.iter().enumerate() {
                    match e {
                        Entry::User { text, refs } => {
                            ui.add_space(6.0);
                            let g = ui.fonts_mut(|f| {
                                f.layout_job(plain_job(text, sans(13.0), t.ink, inner - 16.0))
                            });
                            let chip_h = if refs.is_empty() { 0.0 } else { 22.0 };
                            let (r, _) =
                                ui.allocate_exact_size(vec2(w, g.size().y + 14.0 + chip_h), Sense::hover());
                            let card = Rect::from_min_max(pos2(PAD, r.top()), pos2(w - PAD, r.bottom()));
                            ui.painter().rect_filled(card, 6.0, t.raised);
                            ui.painter()
                                .rect_stroke(card, 6.0, Stroke::new(1.0, t.line), StrokeKind::Inside);
                            ui.painter()
                                .galley(pos2(card.left() + 8.0, card.top() + 7.0), g, t.ink);
                            if !refs.is_empty() {
                                let mut x = card.left() + 8.0;
                                for id in refs {
                                    x += chip(
                                        ui,
                                        &t,
                                        pos2(x, card.bottom() - 25.0),
                                        id,
                                        card.right() - 8.0 - x,
                                    )
                                    .width()
                                        + 4.0;
                                }
                            }
                            places.push((format!("agent:user:{k}"), card));
                            ui.add_space(6.0);
                        }
                        Entry::Agent { text } => {
                            let g = ui.fonts_mut(|f| f.layout_job(rich(text, inner, t.ink, code_ink)));
                            let (r, _) = ui.allocate_exact_size(vec2(w, g.size().y + 10.0), Sense::hover());
                            ui.painter().galley(pos2(PAD, r.top() + 5.0), g, t.ink);
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
                            let ink = t.ink2;
                            super::fg::disclosure(ui.painter(), pos2(PAD + 4.0, r.center().y), open, ink);
                            let dot = match state {
                                ToolState::Running => t.focus.gamma_multiply(0.5 + 0.5 * pulse(ui)),
                                ToolState::Done => ADD_INK,
                                ToolState::Failed => t.error,
                            };
                            ui.painter()
                                .circle_filled(pos2(w - PAD - 4.0, r.center().y), 3.0, dot);
                            let clip = Rect::from_min_max(
                                pos2(PAD + 14.0, r.top()),
                                pos2(w - PAD - 12.0, r.bottom()),
                            );
                            let g = ui.fonts_mut(|f| f.layout_no_wrap(summary.clone(), sans(12.0), ink));
                            ui.painter().with_clip_rect(clip).galley(
                                pos2(clip.left(), r.center().y - g.size().y * 0.5),
                                g,
                                ink,
                            );
                            tools_seen += 1;
                            places.push((format!("agent:tool:{id}"), r));
                            places.push((format!("agent:tool#{tools_seen}"), r));
                            if open {
                                let mut body = input.trim_end().to_string();
                                if !output.trim().is_empty() {
                                    if !body.is_empty() {
                                        body.push_str("\n──\n");
                                    }
                                    let out: Vec<&str> = output.trim_end().lines().take(40).collect();
                                    body.push_str(&out.join("\n"));
                                    if output.lines().count() > 40 {
                                        body.push_str(&format!(
                                            "\n… {} more lines",
                                            output.lines().count() - 40
                                        ));
                                    }
                                }
                                let g = ui.fonts_mut(|f| {
                                    f.layout_job(plain_job(&body, mono(11.0), t.ink2, inner - 12.0))
                                });
                                let (r, _) =
                                    ui.allocate_exact_size(vec2(w, g.size().y + 12.0), Sense::hover());
                                let card = Rect::from_min_max(
                                    pos2(PAD, r.top() + 1.0),
                                    pos2(w - PAD, r.bottom() - 3.0),
                                );
                                ui.painter().rect_filled(card, 4.0, t.field);
                                ui.painter()
                                    .galley(pos2(card.left() + 6.0, card.top() + 4.0), g, t.ink2);
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
                            let shown = lines.len().min(60);
                            let more = lines.len() - shown;
                            let h =
                                26.0 + shown as f32 * DIFF_ROW + if more > 0 { DIFF_ROW } else { 0.0 } + 6.0;
                            let (r, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
                            let card = Rect::from_min_max(pos2(PAD, r.top()), pos2(w - PAD, r.bottom()));
                            let p = ui.painter().clone();
                            p.rect_filled(card, 5.0, t.field);
                            p.rect_stroke(card, 5.0, Stroke::new(1.0, t.line), StrokeKind::Inside);
                            let head = Rect::from_min_size(card.min, vec2(card.width(), 26.0));
                            let name = p.text(
                                pos2(head.left() + 8.0, head.center().y),
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
                        Entry::Note { text, error } => {
                            let ink = if *error { t.error } else { t.ink2 };
                            let g = ui.fonts_mut(|f| f.layout_job(plain_job(text, sans(12.0), ink, inner)));
                            let (r, _) = ui.allocate_exact_size(vec2(w, g.size().y + 8.0), Sense::hover());
                            ui.painter().galley(pos2(PAD, r.top() + 4.0), g, ink);
                            places.push((format!("agent:note:{k}"), r));
                        }
                    }
                }
                if let Some(dir) = &a.trust_ask {
                    trust_card(ui, &t, dir, w, &mut places);
                }
                if a.running && a.trust_ask.is_none() {
                    let dots = ".".repeat(1 + (ui.input(|i| i.time) * 3.0) as usize % 3);
                    let (r, _) = ui.allocate_exact_size(vec2(w, 22.0), Sense::hover());
                    ui.painter().text(
                        pos2(PAD, r.center().y),
                        Align2::LEFT_CENTER,
                        format!("Working{dots}"),
                        sans(12.0),
                        t.ink2,
                    );
                }
                ui.add_space(10.0);
            });
        for id in toggles {
            if !self.agent.open.remove(&id) {
                self.agent.open.insert(id);
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
        self.agent.out.extend(acts);
    }

    fn agent_input(&mut self, ui: &mut Ui, area: Rect) {
        let t = self.theme;
        let p = ui.painter().clone();
        let box_r = Rect::from_min_max(
            area.min + vec2(PAD - 2.0, 4.0),
            area.max - vec2(PAD - 2.0, PAD - 2.0),
        );
        let id = egui::Id::unique(INPUT_ID);
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
        // what's attached, each with its ×
        if !self.agent.refs.is_empty() {
            let mut x = box_r.left() + 6.0;
            let mut drop = None;
            for (k, r) in self.agent.refs.iter().enumerate() {
                let cr = chip(ui, &t, pos2(x, y), r, box_r.right() - 6.0 - x);
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
            y += 26.0;
        }
        let send = focused && ui.input(|i| i.key_pressed(Key::Enter) && !i.modifiers.shift);
        if send {
            ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter));
        }
        let text_r = Rect::from_min_max(
            pos2(box_r.left() + 4.0, y),
            pos2(box_r.right() - 4.0, box_r.bottom() - 28.0),
        );
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(text_r));
        let hint = if self.agent.running {
            "Esc stops the agent"
        } else {
            "Ask the agent"
        };
        let enabled = self.fg.is_some() || self.drawing.is_some();
        let resp = child.add_enabled(
            enabled,
            egui::TextEdit::multiline(&mut self.agent.input)
                .id(id)
                .font(sans(13.0))
                .frame(egui::Frame::NONE)
                .hint_text(hint)
                .return_key(egui::KeyboardShortcut::new(Modifiers::SHIFT, Key::Enter))
                .desired_width(text_r.width())
                .desired_rows(1),
        );
        self.note("agent:input", text_r);
        if ui.input(|i| i.pointer.primary_clicked())
            && ui
                .input(|i| i.pointer.interact_pos())
                .is_some_and(|q| box_r.contains(q) && !text_r.contains(q))
            && enabled
        {
            resp.request_focus();
        }
        if send && enabled {
            self.agent.send();
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
            self.agent.attach(&s);
        }

        let running = self.agent.running;
        let word = if running { "Stop" } else { "Send" };
        let sr = Rect::from_min_size(pos2(box_r.right() - 58.0, row_y), vec2(52.0, 22.0));
        let can = running || (enabled && !self.agent.input.trim().is_empty());
        let s_resp = ui.interact(sr, ui.scope_id().with("agent-send"), Sense::click());
        s_resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, can, word));
        let fill = if running {
            t.error.gamma_multiply(if s_resp.hovered() { 0.9 } else { 0.7 })
        } else if can {
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
            if can { Color32::WHITE } else { t.ink2 },
        );
        self.note("agent:send", sr);
        if s_resp.clicked() && can {
            if running {
                self.agent.out.push(AgentAct::Stop);
            } else {
                self.agent.send();
            }
        }
    }
}

fn pulse(ui: &Ui) -> f32 {
    (ui.input(|i| i.time) as f32 * 5.0).sin() * 0.5 + 0.5
}

/// An attached id: `@id` in a pill, room permitting; its rect.
fn chip(ui: &mut Ui, t: &Theme, at: egui::Pos2, id: &str, room: f32) -> Rect {
    let g = ui.fonts_mut(|f| f.layout_no_wrap(format!("@{id}"), mono(11.0), t.ink));
    let w = (g.size().x + 26.0).min(room.max(40.0));
    let r = Rect::from_min_size(at, vec2(w, 20.0));
    let p = ui.painter().with_clip_rect(r);
    p.rect_filled(r, 10.0, t.on.gamma_multiply(0.6));
    p.galley(pos2(r.left() + 7.0, r.center().y - g.size().y * 0.5), g, t.ink);
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
        Color32::WHITE,
    );
    p.rect_filled(no, 4.0, t.field);
    p.rect_stroke(no, 4.0, Stroke::new(1.0, t.line), StrokeKind::Inside);
    p.text(no.center(), Align2::CENTER_CENTER, "Not now", sans(12.0), t.ink);
    places.push(("agent:trust".into(), card));
    places.push(("agent:trust:yes".into(), yes));
    places.push(("agent:trust:no".into(), no));
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
                refs: vec!["extrude1.end".into()]
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

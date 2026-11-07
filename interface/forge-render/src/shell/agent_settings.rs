//! The Agent tab's settings: which backend runs the agent (Claude Code, Codex, or an OpenAI-compatible
//! endpoint), its model, the endpoint's address, the folders let in. Kept in settings.toml's
//! `[agent]` table; an endpoint's API key lives in the OS keyring, never there. `settings_section`
//! is the Settings window's Agent section (its one line in `settings::SECTIONS`).

use super::theme::{Theme, medium, mono, sans};
use egui::{RichText, Ui};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Kind {
    #[default]
    ClaudeCode,
    Codex,
    OpenAi,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentSettings {
    pub kind: Kind,
    /// `--model` for Claude Code: an alias or a full id; empty for Claude Code's own default.
    pub claude_model: String,
    /// Empty selects the Codex CLI default.
    pub codex_model: String,
    /// An OpenAI-compatible endpoint: its base (up to `/v1`) and model id.
    pub base_url: String,
    pub model: String,
    /// Folders outside ~/projects the agent was let into.
    pub trusted: Vec<String>,
    /// The key that talks to the agent, if one's set; held, or pressed to start and stop.
    pub speech_key: Option<crate::bindings::Chord>,
    pub speech_mode: super::voice::KeyMode,
    /// What's said is sent when it's done, not left in the box to edit.
    pub speech_send: bool,
}

impl Default for AgentSettings {
    fn default() -> AgentSettings {
        AgentSettings {
            kind: Kind::ClaudeCode,
            claude_model: String::new(),
            codex_model: String::new(),
            base_url: "http://localhost:8080/v1".into(),
            model: String::new(),
            trusted: Vec::new(),
            speech_key: None,
            speech_mode: super::voice::KeyMode::Hold,
            speech_send: false,
        }
    }
}

/// What a settings change asks of the host.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SettingsAct {
    /// Write `agent.toml`; the next message uses what's set.
    Changed,
    /// Store this key for this endpoint in the keyring; an empty key forgets it.
    SaveKey { url: String, key: String },
}

/// The key field: typed, never shown back; whether the keyring holds one for the endpoint.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KeyField {
    pub typed: String,
    pub stored: Option<bool>,
    /// The keyring's answer to the last save, when it refused.
    pub error: Option<String>,
}

impl AgentSettings {
    /// The header's words: the backend and its model.
    pub fn label(&self) -> String {
        match self.kind {
            Kind::ClaudeCode => match self.claude_model.trim() {
                "" => "Claude Code".into(),
                m => format!("Claude Code · {m}"),
            },
            Kind::Codex => match self.codex_model.trim() {
                "" => "Codex".into(),
                m => format!("Codex · {m}"),
            },
            Kind::OpenAi => {
                let host = host_of(&self.base_url);
                match self.model.trim() {
                    "" => format!("{host} · no model"),
                    m => format!("{host} · {m}"),
                }
            }
        }
    }

    pub fn to_toml(&self) -> String {
        let kind = match self.kind {
            Kind::ClaudeCode => "claude-code",
            Kind::Codex => "codex",
            Kind::OpenAi => "openai",
        };
        let trusted: Vec<String> = self.trusted.iter().map(|t| quote(t)).collect();
        format!(
            "backend = {}\nclaude_model = {}\ncodex_model = {}\nbase_url = {}\nmodel = {}\ntrusted = [{}]\n",
            quote(kind),
            quote(&self.claude_model),
            quote(&self.codex_model),
            quote(&self.base_url),
            quote(&self.model),
            trusted.join(", ")
        )
    }

    /// What a file says, defaults for what it doesn't.
    pub fn from_toml(text: &str) -> AgentSettings {
        let mut s = AgentSettings::default();
        for line in text.lines() {
            let Some((k, v)) = line.split_once('=') else {
                continue;
            };
            let v = unquote(v.trim());
            match k.trim() {
                "backend" => {
                    s.kind = match v.as_str() {
                        "openai" => Kind::OpenAi,
                        "codex" => Kind::Codex,
                        _ => Kind::ClaudeCode,
                    }
                }
                "claude_model" => s.claude_model = v,
                "codex_model" => s.codex_model = v,
                "base_url" => s.base_url = v,
                "model" => s.model = v,
                "trusted" => s.trusted = strings(line.split_once('=').map_or("", |x| x.1)),
                _ => {}
            }
        }
        s
    }
}

pub fn host_of(url: &str) -> String {
    let rest = url.split_once("://").map_or(url, |x| x.1);
    rest.split('/').next().unwrap_or(rest).to_string()
}

// the key field's state and the keyring saves asked for, between the Settings window that
// draws them and the host that runs them (the window's section is a plain fn of the settings)
thread_local! {
    static KEY: std::cell::RefCell<KeyField> = Default::default();
    static ASKED: std::cell::RefCell<Vec<SettingsAct>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// The keyring saves the section asked for since last time.
pub fn take_asked() -> Vec<SettingsAct> {
    ASKED.with(|a| std::mem::take(&mut *a.borrow_mut()))
}

/// The key field, to read or set (whether the keyring holds a key, its last refusal).
pub fn with_key<R>(f: impl FnOnce(&mut KeyField) -> R) -> R {
    KEY.with(|k| f(&mut k.borrow_mut()))
}

const TABLE: &str = "agent";

impl AgentSettings {
    /// What settings.toml's `[agent]` table says, defaults for what it doesn't.
    pub fn from_extra(t: &crate::bindings::Toml) -> AgentSettings {
        let mut s = AgentSettings::default();
        let get = |k: &str| t.get(TABLE, k).map(str::to_string);
        if let Some(v) = get("backend") {
            s.kind = match v.as_str() {
                "openai" => Kind::OpenAi,
                "codex" => Kind::Codex,
                _ => Kind::ClaudeCode,
            };
        }
        if let Some(v) = get("claude_model") {
            s.claude_model = v;
        }
        if let Some(v) = get("codex_model") {
            s.codex_model = v;
        }
        if let Some(v) = get("base_url") {
            s.base_url = v;
        }
        if let Some(v) = get("model") {
            s.model = v;
        }
        // trusted.0, trusted.1, ...: one folder a key, never one string to split
        s.trusted = (0..)
            .map_while(|k| get(&format!("trusted.{k}")))
            .filter(|d| trustable(d))
            .collect();
        s.speech_key = get("speech_key").and_then(|k| crate::bindings::Chord::parse(&k));
        s.speech_mode = match get("speech_mode").as_deref() {
            Some("toggle") => super::voice::KeyMode::Toggle,
            _ => super::voice::KeyMode::Hold,
        };
        s.speech_send = get("speech_send").as_deref() == Some("true");
        s
    }

    /// The `[agent]` table, written whole.
    pub fn write_extra(&self, t: &mut crate::bindings::Toml) {
        t.entries.retain(|e| e.0 != TABLE);
        let kind = match self.kind {
            Kind::ClaudeCode => "claude-code",
            Kind::Codex => "codex",
            Kind::OpenAi => "openai",
        };
        t.set(TABLE, "backend", kind);
        t.set(TABLE, "claude_model", self.claude_model.clone());
        t.set(TABLE, "codex_model", self.codex_model.clone());
        t.set(TABLE, "base_url", self.base_url.clone());
        t.set(TABLE, "model", self.model.clone());
        for (k, d) in self.trusted.iter().filter(|d| trustable(d)).enumerate() {
            t.set(TABLE, &format!("trusted.{k}"), d.clone());
        }
        if let Some(k) = self.speech_key {
            t.set(TABLE, "speech_key", k.to_string());
        }
        let mode = match self.speech_mode {
            super::voice::KeyMode::Hold => "hold",
            super::voice::KeyMode::Toggle => "toggle",
        };
        t.set(TABLE, "speech_mode", mode);
        t.set(
            TABLE,
            "speech_send",
            if self.speech_send { "true" } else { "false" },
        );
    }

    /// The table's there: written by this, or an older agent.toml already moved in.
    pub fn in_extra(t: &crate::bindings::Toml) -> bool {
        t.entries.iter().any(|e| e.0 == TABLE)
    }
}

/// The Settings window's Agent section.
pub fn settings_section(
    ui: &mut Ui,
    t: &Theme,
    s: &mut super::settings::Settings,
    _w: &mut super::settings::Window,
    _places: &mut Vec<(String, egui::Rect)>,
) {
    let mut a = AgentSettings::from_extra(&s.extra);
    let acts = with_key(|k| section(ui, t, &mut a, k));
    for act in acts {
        match act {
            SettingsAct::Changed => a.write_extra(&mut s.extra),
            other => ASKED.with(|q| q.borrow_mut().push(other)),
        }
    }
}

/// A folder fit to trust: absolute, no control characters (a newline would be a second entry).
pub fn trustable(dir: &str) -> bool {
    dir.starts_with('/') && !dir.chars().any(char::is_control)
}

/// A TOML array of strings: each quoted one, the untrustable left out.
fn strings(v: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut it = v.trim().trim_start_matches('[').chars().peekable();
    while let Some(c) = it.next() {
        if c != '"' {
            continue;
        }
        let mut s = String::new();
        while let Some(c) = it.next() {
            match c {
                '"' => break,
                '\\' => match it.next() {
                    Some('n') => s.push('\n'),
                    Some(c) => s.push(c),
                    None => {}
                },
                c => s.push(c),
            }
        }
        if trustable(&s) {
            out.push(s);
        }
    }
    out
}

fn quote(s: &str) -> String {
    let mut o = String::from('"');
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

fn unquote(v: &str) -> String {
    let Some(inner) = v.strip_prefix('"').and_then(|v| v.strip_suffix('"')) else {
        return v.to_string();
    };
    let mut o = String::new();
    let mut it = inner.chars();
    while let Some(c) = it.next() {
        if c == '\\' {
            match it.next() {
                Some('n') => o.push('\n'),
                Some(c) => o.push(c),
                None => {}
            }
        } else {
            o.push(c);
        }
    }
    o
}

/// The settings, editable: the backend, its model, the endpoint and its key.
pub fn section(ui: &mut Ui, t: &Theme, s: &mut AgentSettings, key: &mut KeyField) -> Vec<SettingsAct> {
    let mut out = Vec::new();
    let was = s.clone();
    ui.spacing_mut().item_spacing.y = 6.0;
    ui.label(RichText::new("Backend").font(medium(12.0)).color(t.ink2));
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 16.0;
        ui.radio_value(&mut s.kind, Kind::ClaudeCode, "Claude Code");
        ui.radio_value(&mut s.kind, Kind::Codex, "Codex");
        ui.radio_value(&mut s.kind, Kind::OpenAi, "OpenAI-compatible");
    });
    ui.add_space(4.0);
    let field = |ui: &mut Ui, label: &str, v: &mut String, hint: &str| {
        ui.label(RichText::new(label).font(sans(12.0)).color(t.ink2));
        ui.add(
            egui::TextEdit::singleline(v)
                .font(mono(12.0))
                .hint_text(hint)
                .desired_width(f32::INFINITY),
        )
    };
    match s.kind {
        Kind::ClaudeCode => {
            field(ui, "Model", &mut s.claude_model, "Claude Code's default")
                .on_hover_text("Passed as --model: an alias (opus, sonnet, haiku) or a full model id");
        }
        Kind::Codex => {
            field(ui, "Model", &mut s.codex_model, "Codex default");
            ui.label(
                RichText::new("Uses your Codex CLI login. Sign in with: codex login")
                    .font(sans(12.0))
                    .color(t.ink2),
            );
        }
        Kind::OpenAi => {
            field(ui, "Base URL", &mut s.base_url, "https://api.openai.com/v1");
            field(ui, "Model", &mut s.model, "gpt-5, qwen3.6-35b-a3b, ...");
            ui.label(RichText::new("API key").font(sans(12.0)).color(t.ink2));
            ui.horizontal(|ui| {
                let hint = match key.stored {
                    Some(true) => "stored in the keyring",
                    Some(false) => "none (a local server needs none)",
                    None => "",
                };
                ui.add(
                    egui::TextEdit::singleline(&mut key.typed)
                        .password(true)
                        .font(mono(12.0))
                        .hint_text(hint)
                        .desired_width((ui.available_width() - 120.0).max(80.0)),
                );
                if ui
                    .add_enabled(!key.typed.is_empty(), egui::Button::new("Save key"))
                    .clicked()
                {
                    out.push(SettingsAct::SaveKey {
                        url: s.base_url.trim().to_string(),
                        key: std::mem::take(&mut key.typed),
                    });
                }
                if key.stored == Some(true) && ui.button("Forget").clicked() {
                    out.push(SettingsAct::SaveKey {
                        url: s.base_url.trim().to_string(),
                        key: String::new(),
                    });
                }
            });
            if let Some(e) = &key.error {
                ui.label(RichText::new(e).font(sans(12.0)).color(t.error));
            }
        }
    }
    speech(ui, t, s);
    if *s != was {
        out.push(SettingsAct::Changed);
        if s.base_url != was.base_url {
            key.stored = None;
        }
    }
    out
}

thread_local! {
    static CAPTURING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Speech: the key that talks, how it works, and whether what's said is sent.
fn speech(ui: &mut Ui, t: &Theme, s: &mut AgentSettings) {
    use super::voice::KeyMode;
    ui.add_space(10.0);
    ui.label(RichText::new("Speech").font(medium(12.0)).color(t.ink2));
    ui.horizontal(|ui| {
        ui.label(RichText::new("Key").font(sans(12.0)).color(t.ink2));
        let capturing = CAPTURING.with(|c| c.get());
        let words = match (capturing, s.speech_key) {
            (true, _) => "Press a key…".to_string(),
            (false, Some(k)) => k.to_string(),
            (false, None) => "None".to_string(),
        };
        let b =
            ui.add(egui::Button::new(RichText::new(words).font(mono(12.0))).min_size(egui::vec2(120.0, 0.0)));
        if b.clicked() {
            CAPTURING.with(|c| c.set(!capturing));
        }
        if capturing {
            let got = ui.input_mut(|i| {
                let k = i.events.iter().find_map(|e| match e {
                    egui::Event::Key {
                        key,
                        pressed: true,
                        modifiers,
                        ..
                    } => Some((*key, *modifiers)),
                    _ => None,
                });
                if k.is_some() {
                    i.events.clear();
                }
                k
            });
            if let Some((k, m)) = got {
                CAPTURING.with(|c| c.set(false));
                if k != egui::Key::Escape {
                    s.speech_key = Some(crate::bindings::Chord::from_event(k, m));
                }
            }
        }
        if s.speech_key.is_some() && ui.button("Clear").clicked() {
            s.speech_key = None;
        }
    });
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 16.0;
        ui.radio_value(&mut s.speech_mode, KeyMode::Hold, "Hold to talk");
        ui.radio_value(&mut s.speech_mode, KeyMode::Toggle, "Press to start and stop");
    });
    ui.checkbox(&mut s.speech_send, "Send when done");
}

/// The OpenAI-compatible endpoint can't run without these.
pub fn missing(s: &AgentSettings) -> Option<&'static str> {
    match s.kind {
        Kind::ClaudeCode | Kind::Codex => None,
        Kind::OpenAi if s.base_url.trim().is_empty() => Some("Set the endpoint's base URL in Agent settings"),
        Kind::OpenAi if s.model.trim().is_empty() => Some("Set a model for the endpoint in Agent settings"),
        Kind::OpenAi => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toml_round_trip() {
        let s = AgentSettings {
            kind: Kind::OpenAi,
            claude_model: "opus".into(),
            codex_model: String::new(),
            base_url: "http://node1:8080/v1".into(),
            model: "qwen \"3.6\"".into(),
            trusted: vec!["/mnt/usb/parts".into(), "/tmp/x, \"y\"".into()],
            ..AgentSettings::default()
        };
        assert_eq!(AgentSettings::from_toml(&s.to_toml()), s);
        assert_eq!(s.label(), "node1:8080 · qwen \"3.6\"");
        assert_eq!(AgentSettings::from_toml(""), AgentSettings::default());
        assert!(!s.to_toml().contains("key"));
        // a newline can't make a second entry: an untrustable path is dropped on reading
        let sneaky = AgentSettings {
            trusted: vec!["/ok".into(), "/a\n/etc".into()],
            ..AgentSettings::default()
        };
        assert_eq!(AgentSettings::from_toml(&sneaky.to_toml()).trusted, ["/ok"]);
        assert!(!trustable("/a\nb") && !trustable("rel") && trustable("/home/x y"));
    }

    #[test]
    fn settings_tomls_agent_table_round_trips_and_a_newline_adds_no_folder() {
        let a = AgentSettings {
            kind: Kind::OpenAi,
            claude_model: "opus".into(),
            codex_model: String::new(),
            base_url: "http://node1:8080/v1".into(),
            model: "qwen3.6-35b-a3b".into(),
            trusted: vec!["/mnt/usb/parts".into(), "/tmp/x y".into(), "/a\n/etc".into()],
            speech_key: crate::bindings::Chord::parse("Ctrl+Space"),
            speech_mode: crate::shell::voice::KeyMode::Toggle,
            speech_send: true,
        };
        let mut s = crate::shell::settings::Settings::default();
        a.write_extra(&mut s.extra);
        let text = s.to_toml();
        assert!(text.contains("[agent]"), "{text}");
        let back = crate::shell::settings::Settings::from_toml(&text);
        let got = AgentSettings::from_extra(&back.extra);
        assert_eq!(got.trusted, ["/mnt/usb/parts", "/tmp/x y"]);
        assert_eq!(
            AgentSettings {
                trusted: vec![],
                ..got
            },
            AgentSettings { trusted: vec![], ..a }
        );
        assert_eq!(
            AgentSettings::from_extra(&Default::default()),
            AgentSettings::default()
        );
    }
}

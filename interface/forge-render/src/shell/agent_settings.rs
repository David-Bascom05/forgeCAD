//! The Agent tab's settings: which backend runs the agent (Claude Code, or any OpenAI-compatible
//! endpoint), its model, the endpoint's address. Kept in `agent.toml` beside `settings.toml`; an
//! endpoint's API key lives in the OS keyring, never here. `section` is the Settings window's
//! Agent tab and the Agent tab's own settings window both.

use super::theme::{Theme, medium, mono, sans};
use egui::{RichText, Ui};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Kind {
    #[default]
    ClaudeCode,
    OpenAi,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentSettings {
    pub kind: Kind,
    /// `--model` for Claude Code: an alias or a full id; empty for Claude Code's own default.
    pub claude_model: String,
    /// An OpenAI-compatible endpoint: its base (up to `/v1`) and model id.
    pub base_url: String,
    pub model: String,
}

impl Default for AgentSettings {
    fn default() -> AgentSettings {
        AgentSettings {
            kind: Kind::ClaudeCode,
            claude_model: String::new(),
            base_url: "http://localhost:8080/v1".into(),
            model: String::new(),
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
            Kind::OpenAi => "openai",
        };
        format!(
            "backend = {}\nclaude_model = {}\nbase_url = {}\nmodel = {}\n",
            quote(kind),
            quote(&self.claude_model),
            quote(&self.base_url),
            quote(&self.model)
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
                    s.kind = if v == "openai" {
                        Kind::OpenAi
                    } else {
                        Kind::ClaudeCode
                    }
                }
                "claude_model" => s.claude_model = v,
                "base_url" => s.base_url = v,
                "model" => s.model = v,
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
        ui.radio_value(&mut s.kind, Kind::ClaudeCode, "Claude Code");
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
    if *s != was {
        out.push(SettingsAct::Changed);
        if s.base_url != was.base_url {
            key.stored = None;
        }
    }
    out
}

/// The OpenAI-compatible endpoint can't run without these.
pub fn missing(s: &AgentSettings) -> Option<&'static str> {
    match s.kind {
        Kind::ClaudeCode => None,
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
            base_url: "http://node1:8080/v1".into(),
            model: "qwen \"3.6\"".into(),
        };
        assert_eq!(AgentSettings::from_toml(&s.to_toml()), s);
        assert_eq!(s.label(), "node1:8080 · qwen \"3.6\"");
        assert_eq!(AgentSettings::from_toml(""), AgentSettings::default());
        assert!(!s.to_toml().contains("key"));
    }
}

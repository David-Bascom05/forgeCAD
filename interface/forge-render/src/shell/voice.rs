//! Talking to the agent: the mic button in the Agent tab's message box, its key (held to talk,
//! or pressed to start and stop), and what the status bar shows while it listens. What's said
//! goes into the message box as it's heard, to edit before sending (or sent when done, if the
//! settings say so). The host runs the recogniser (`crate::speech`) and does what `VoiceAct`s ask.

use super::Shell;
use super::icons;
use super::theme::{Theme, sans};
use egui::{Color32, Rect, Sense, Stroke, Ui, pos2, vec2};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum VoiceState {
    #[default]
    Idle,
    /// Asked to listen; the recogniser's starting.
    Starting,
    Listening,
    /// Done listening; the last words are being made into text.
    Transcribing,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VoiceAct {
    Start,
    Stop,
    Cancel,
}

#[derive(Clone, Debug, Default)]
pub struct Voice {
    pub state: VoiceState,
    /// The microphone's loudness now, 0..1.
    pub level: f32,
    /// Why speech can't run (no recogniser, no model, no microphone), once that's known.
    pub unavailable: Option<String>,
    /// What the box held when listening began: what's heard goes after it.
    pub prefix: String,
    pub out: Vec<VoiceAct>,
    /// The key's held down (push to talk).
    pub held: bool,
    /// The last one's time from the end of speech to its text, and the engine.
    pub last_ms: Option<f64>,
    pub engine: Option<String>,
}

impl Voice {
    /// The button or the key in toggle mode: listen, or stop listening.
    pub fn press(&mut self) {
        match self.state {
            VoiceState::Idle if self.unavailable.is_none() => self.out.push(VoiceAct::Start),
            VoiceState::Starting | VoiceState::Listening => self.out.push(VoiceAct::Stop),
            _ => {}
        }
    }

    pub fn busy(&self) -> bool {
        self.state != VoiceState::Idle
    }

    /// The box's text with what's heard so far after what was typed.
    pub fn with_heard(&self, heard: &str) -> String {
        let heard = heard.trim();
        if heard.is_empty() {
            return self.prefix.clone();
        }
        if self.prefix.is_empty() || self.prefix.ends_with(char::is_whitespace) {
            format!("{}{heard}", self.prefix)
        } else {
            format!("{} {heard}", self.prefix)
        }
    }
}

/// How the key works: held to talk, or pressed once to start and again to stop.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum KeyMode {
    #[default]
    Hold,
    Toggle,
}

/// The ink a state draws in.
fn ink(t: &Theme, v: &Voice) -> Color32 {
    match v.state {
        VoiceState::Listening => t.error,
        VoiceState::Starting | VoiceState::Transcribing => t.focus,
        VoiceState::Idle if v.unavailable.is_some() => t.ink2.gamma_multiply(0.5),
        VoiceState::Idle => t.ink,
    }
}

/// A level meter: five bars, as many lit as it's loud.
fn meter(p: &egui::Painter, r: Rect, level: f32, on: Color32, off: Color32) {
    let n = 5;
    let w = r.width() / n as f32;
    for k in 0..n {
        let lit = level * n as f32 > k as f32 + 0.15;
        let h = r.height() * (0.35 + 0.65 * (k as f32 + 1.0) / n as f32);
        let b = Rect::from_min_max(
            pos2(r.left() + k as f32 * w + 0.5, r.bottom() - h),
            pos2(r.left() + (k as f32 + 1.0) * w - 1.0, r.bottom()),
        );
        p.rect_filled(b, 1.0, if lit { on } else { off });
    }
}

impl Shell {
    /// The mic button at `r` in the message box's bottom row; while listening, the meter left of it.
    pub(super) fn mic_button(&mut self, ui: &mut Ui, r: Rect, enabled: bool) {
        let t = self.theme;
        let v = &self.agent.voice;
        let p = ui.painter().clone();
        let resp = ui.interact(r, ui.scope_id().with("agent-mic"), Sense::click());
        let tip = match (&v.unavailable, v.state) {
            (Some(why), _) => format!("Speech isn't available: {why}"),
            (None, VoiceState::Listening | VoiceState::Starting) => "Stop listening".into(),
            (None, VoiceState::Transcribing) => "Transcribing".into(),
            (None, VoiceState::Idle) => match self.agent.settings.speech_key {
                Some(k) => format!("Talk ({k})"),
                None => "Talk".into(),
            },
        };
        let can = enabled && v.unavailable.is_none() && v.state != VoiceState::Transcribing;
        let resp = resp.on_hover_text(tip);
        resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, can, "Talk"));
        let state = v.state;
        if state == VoiceState::Listening {
            p.rect_filled(r, 4.0, t.error.gamma_multiply(0.18));
        } else if resp.hovered() && can {
            p.rect_filled(r, 4.0, t.hover);
        }
        let ink = if enabled {
            ink(&t, v)
        } else {
            t.ink2.gamma_multiply(0.5)
        };
        icons::draw(
            &p,
            pos2(r.center().x - 8.0, r.center().y - 8.0),
            16.0,
            "mic",
            1.5,
            ink,
        );
        if state == VoiceState::Starting || state == VoiceState::Transcribing {
            let a = (ui.input(|i| i.time) * 4.0) as f32;
            p.circle_stroke(
                r.center(),
                10.0,
                Stroke::new(1.0, ink.gamma_multiply(0.3 + 0.3 * a.sin().abs())),
            );
        }
        self.note("agent:mic", r);
        if state == VoiceState::Listening {
            let m = Rect::from_min_size(pos2(r.left() - 26.0, r.top() + 5.0), vec2(22.0, 12.0));
            meter(&p, m, self.agent.voice.level, t.error, t.line);
            self.note("agent:mic:level", m);
        }
        if resp.clicked() && can {
            self.agent.voice.press();
        }
        if self.agent.voice.busy() {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(50));
        }
    }

    /// The speech key, wherever the keys are (in the message box too): held to talk, or pressed
    /// to start and stop. It's taken out of the frame's events so nothing else reads it.
    pub(super) fn voice_keys(&mut self, ctx: &egui::Context) {
        let Some(chord) = self.agent.settings.speech_key else {
            return;
        };
        if self.prefs.capture.is_some() || self.prefs.open {
            return;
        }
        let enabled = !self.agent.running;
        let mode = self.agent.settings.speech_mode;
        let mut down = false;
        let mut up = false;
        ctx.input_mut(|i| {
            i.events.retain(|e| match e {
                egui::Event::Key {
                    key,
                    pressed,
                    repeat,
                    modifiers,
                    ..
                } if *key == chord.key && (!*pressed || chord.matches(*key, *modifiers)) => {
                    if *pressed && !*repeat {
                        down = true;
                    } else if !*pressed {
                        up = true;
                    }
                    false
                }
                // the text a chord's key would type (Space) isn't typed
                egui::Event::Text(s)
                    if chord.key == egui::Key::Space && s == " " && (down || self.agent.voice.held) =>
                {
                    false
                }
                _ => true,
            })
        });
        let v = &mut self.agent.voice;
        if down && enabled && v.unavailable.is_none() {
            match mode {
                KeyMode::Hold if !v.held && v.state == VoiceState::Idle => {
                    v.held = true;
                    v.out.push(VoiceAct::Start);
                }
                KeyMode::Toggle => v.press(),
                _ => {}
            }
            self.agent.tab = super::agent::SideTab::Agent;
        }
        let v = &mut self.agent.voice;
        if up && v.held {
            v.held = false;
            if matches!(v.state, VoiceState::Starting | VoiceState::Listening)
                || v.out.contains(&VoiceAct::Start)
            {
                v.out.push(VoiceAct::Stop);
            }
        }
    }

    /// While it listens or transcribes, the status bar says so (with the meter), from `x`; a
    /// click there stops listening. Its right edge.
    pub(super) fn voice_status(&mut self, ui: &mut Ui, x: f32, bar: Rect) -> f32 {
        let v = &self.agent.voice;
        let word = match v.state {
            VoiceState::Idle => return x,
            VoiceState::Starting => "Starting speech",
            VoiceState::Listening => "Listening",
            VoiceState::Transcribing => "Transcribing",
        };
        let p = ui.painter().clone();
        let g = ui.fonts_mut(|f| f.layout_no_wrap(word.into(), sans(12.0), Color32::WHITE));
        let listening = v.state == VoiceState::Listening;
        let w = 22.0 + g.size().x + if listening { 30.0 } else { 6.0 };
        let r = Rect::from_min_size(pos2(x, bar.top() + 2.0), vec2(w, bar.height() - 4.0));
        let resp = ui.interact(r, ui.scope_id().with("status-voice"), Sense::click());
        if resp.hovered() {
            p.rect_filled(r, 3.0, Color32::from_white_alpha(28));
        }
        let ink = if listening {
            Color32::from_rgb(0xFF, 0x8A, 0x80)
        } else {
            Color32::WHITE
        };
        icons::draw(
            &p,
            pos2(r.left() + 3.0, r.center().y - 7.0),
            14.0,
            "mic",
            1.4,
            ink,
        );
        p.galley(
            pos2(r.left() + 20.0, r.center().y - g.size().y * 0.5),
            g.clone(),
            Color32::WHITE,
        );
        if listening {
            let m = Rect::from_min_size(
                pos2(r.left() + 24.0 + g.size().x, r.center().y - 6.0),
                vec2(22.0, 12.0),
            );
            meter(&p, m, v.level, ink, Color32::from_white_alpha(50));
        }
        self.note("status:voice", r);
        if resp.clicked()
            && matches!(
                self.agent.voice.state,
                VoiceState::Listening | VoiceState::Starting
            )
        {
            self.agent.voice.out.push(VoiceAct::Stop);
        }
        r.right()
    }

    /// The open document's names, for the recogniser to expect and the transcript to be written
    /// with: the file, its features, parameters and an assembly's occurrences and parts.
    pub fn speech_names(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let mut add = |s: &str| {
            let s = s.trim();
            if !s.is_empty() && !out.iter().any(|o| o == s) {
                out.push(s.to_string());
            }
        };
        if let Some(fg) = &self.fg {
            add(std::path::Path::new(&fg.file)
                .file_stem()
                .map_or("", |s| s.to_str().unwrap_or("")));
            for f in &fg.feats {
                add(&f.id);
            }
            for p in &fg.params {
                add(&p.key.name);
            }
            if let Some(a) = &fg.asm {
                for o in &a.occ {
                    add(&o.path);
                    add(std::path::Path::new(&o.file)
                        .file_stem()
                        .map_or("", |s| s.to_str().unwrap_or("")));
                }
                for n in &a.tree.nodes {
                    add(&n.label);
                }
            }
        }
        if let Some(d) = &self.drawing {
            add(&d.name());
        }
        out.truncate(400);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn button_and_key_states() {
        let mut v = Voice::default();
        v.press();
        assert_eq!(v.out, [VoiceAct::Start]);
        v.out.clear();
        v.state = VoiceState::Listening;
        v.press();
        assert_eq!(v.out, [VoiceAct::Stop]);
        v.out.clear();
        v.state = VoiceState::Transcribing;
        v.press();
        assert!(v.out.is_empty());
        v.state = VoiceState::Idle;
        v.unavailable = Some("no model".into());
        v.press();
        assert!(v.out.is_empty());
        v.prefix = "make it".into();
        assert_eq!(v.with_heard(" thicker "), "make it thicker");
        v.prefix = "make it ".into();
        assert_eq!(v.with_heard("thicker"), "make it thicker");
        assert_eq!(v.with_heard(""), "make it ");
    }
}

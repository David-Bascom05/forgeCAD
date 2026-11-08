//! The Agent tab's speech, host side: `forge-speech` started on first use and kept running (its
//! model warm), told the open document's names each time it listens, what it hears put in the
//! message box. `FORGECAD_SPEECH_WAV` puts a WAV file in the microphone's place.

use super::Gui;
use crate::shell::voice::{VoiceAct, VoiceState};
use crate::speech::{Heard, Speech, words};
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, channel};

#[derive(Default)]
pub struct SpeechHost {
    proc: Option<Speech>,
    /// `forge-speech`, unless a test says otherwise.
    pub exe: Option<PathBuf>,
    /// A WAV file heard in the microphone's place.
    pub wav: Option<PathBuf>,
    checked: bool,
    check: Option<Receiver<Option<String>>>,
    /// The names it was told this time, for writing what it heard.
    names: Vec<String>,
}

/// Whether `forge-speech` can run here (its library, model and a microphone or the WAV): None, or why not.
fn check(exe: &std::path::Path) -> Option<String> {
    let out = std::process::Command::new(exe).arg("--check").output();
    match out {
        Ok(o) if o.status.success() => None,
        Ok(o) => Some(
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .next()
                .unwrap_or("the recogniser can't start")
                .to_string(),
        ),
        Err(e) => Some(format!("forge-speech can't run: {e}")),
    }
}

impl Gui {
    /// Whether speech can run here is known.
    pub fn speech_checked(&self) -> bool {
        self.speech.checked && self.speech.check.is_none()
    }

    /// Before the frame: whether speech can run (asked once, off the frame's thread), what's heard.
    pub(crate) fn take_speech(&mut self) {
        let h = &mut self.speech;
        if !h.checked {
            h.checked = true;
            if h.wav.is_none() {
                h.wav = std::env::var_os("FORGECAD_SPEECH_WAV").map(PathBuf::from);
            }
            match h.exe.clone().or_else(crate::speech::find) {
                None => {
                    self.shell.agent.voice.unavailable = Some("forge-speech isn't installed".into());
                }
                Some(exe) => {
                    h.exe = Some(exe.clone());
                    let (tx, rx) = channel();
                    let waker = self.waker.clone();
                    std::thread::spawn(move || {
                        let _ = tx.send(check(&exe));
                        waker();
                    });
                    h.check = Some(rx);
                }
            }
        }
        if let Some(rx) = &h.check
            && let Ok(why) = rx.try_recv()
        {
            h.check = None;
            self.shell.agent.voice.unavailable = why;
        }
        let heard = h.proc.as_ref().map(Speech::heard).unwrap_or_default();
        for e in heard {
            self.speech_heard(e);
        }
    }

    fn speech_heard(&mut self, e: Heard) {
        let v = &mut self.shell.agent.voice;
        match e {
            Heard::Ready { engine, .. } => v.engine = Some(engine),
            Heard::Level(l) => {
                if v.state == VoiceState::Starting {
                    v.state = VoiceState::Listening;
                }
                // up at once, down slowly: the meter doesn't flicker
                v.level = if l > v.level { l } else { v.level * 0.7 + l * 0.3 };
            }
            Heard::Partial(text) => {
                if matches!(
                    v.state,
                    VoiceState::Starting | VoiceState::Listening | VoiceState::Transcribing
                ) {
                    if v.state == VoiceState::Starting {
                        v.state = VoiceState::Listening;
                    }
                    let text = words::tidy(&text, &self.speech.names);
                    self.shell.agent.input = v.with_heard(&text);
                }
            }
            Heard::Final { text, ms } => {
                if v.state == VoiceState::Idle {
                    return;
                }
                let text = words::tidy(&text, &self.speech.names);
                self.shell.agent.input = v.with_heard(&text);
                v.state = VoiceState::Idle;
                v.level = 0.0;
                v.last_ms = Some(ms);
                if self.shell.agent.settings.speech_send && !text.is_empty() {
                    self.shell.agent.send();
                }
            }
            Heard::Error(e) => {
                v.state = VoiceState::Idle;
                v.level = 0.0;
                self.shell.status = format!("Speech: {e}");
            }
            Heard::Gone(why) => {
                self.speech.proc = None;
                v.state = VoiceState::Idle;
                v.level = 0.0;
                v.held = false;
                v.unavailable = Some(if why.is_empty() {
                    "the recogniser stopped".into()
                } else {
                    why
                });
            }
        }
    }

    /// After the frame: the button's and the key's asks.
    pub(crate) fn speech_acts(&mut self) {
        let acts = std::mem::take(&mut self.shell.agent.voice.out);
        for a in acts {
            match a {
                VoiceAct::Start => self.speech_start(),
                VoiceAct::Stop => {
                    let v = &mut self.shell.agent.voice;
                    if let Some(p) = &mut self.speech.proc
                        && matches!(v.state, VoiceState::Starting | VoiceState::Listening)
                    {
                        p.stop();
                        v.state = VoiceState::Transcribing;
                        v.level = 0.0;
                    }
                }
                VoiceAct::Cancel => {
                    let v = &mut self.shell.agent.voice;
                    if let Some(p) = &mut self.speech.proc {
                        p.cancel();
                    }
                    if v.busy() {
                        self.shell.agent.input = v.prefix.clone();
                    }
                    v.state = VoiceState::Idle;
                    v.level = 0.0;
                    v.held = false;
                }
            }
        }
    }

    fn speech_start(&mut self) {
        if self.shell.agent.voice.busy() || self.shell.agent.voice.unavailable.is_some() {
            return;
        }
        if self.speech.proc.is_none() {
            let Some(exe) = self.speech.exe.clone() else {
                return;
            };
            match Speech::start(&exe, self.waker.clone()) {
                Ok(p) => self.speech.proc = Some(p),
                Err(e) => {
                    self.shell.agent.voice.unavailable = Some(format!("forge-speech can't run: {e}"));
                    return;
                }
            }
        }
        let names = self.shell.speech_names();
        let wav = self.speech.wav.clone();
        let Some(p) = &mut self.speech.proc else { return };
        p.names(&words::hints(&names));
        p.listen(wav.as_deref());
        self.speech.names = names;
        let v = &mut self.shell.agent.voice;
        v.prefix = self.shell.agent.input.clone();
        v.state = VoiceState::Starting;
        v.level = 0.0;
    }
}

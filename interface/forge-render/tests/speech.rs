//! Talking to the agent: the mic button and the speech key, through a recogniser process into
//! the message box. A stand-in recogniser (a shell script saying the protocol's lines) is
//! quick; the real one, `forge-speech` with its model, hears a WAV file (by hand: it needs the
//! model, `FORGECAD_SPEECH` and `FORGECAD_SPEECH_TEST_WAV`).

use forge_render::bindings::Chord;
use forge_render::gui::{Gui, Headless};
use forge_render::shell::voice::{KeyMode, VoiceState};
use std::path::{Path, PathBuf};

const PART: &str = "param sk1.x1 = -40
param sk1.y2 = -30
param sk1.w3 = 80
param sk1.h4 = 60
sketch sk1(on: xy) {
  r1 = rect(corner: (x1, y2), size: (w3, h4))
}
param extrude1.d = 12
extrude1 = extrude(sk1, d)
";

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("forgecad-speech-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn wait(h: &mut Headless, what: impl Fn(&Gui) -> bool) -> bool {
    let t = std::time::Instant::now();
    while t.elapsed().as_secs() < 60 {
        h.step();
        if what(&h.gui) {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    false
}

/// A recogniser that hears "set extrude one depth to twelve point five millimetres", as
/// whisper writes it, and says what it was told.
fn fake(dir: &Path) -> PathBuf {
    let p = dir.join("forge-speech");
    let log = dir.join("said.txt");
    std::fs::write(
        &p,
        format!(
            "#!/bin/sh\n[ \"$1\" = --check ] && {{ echo ok; exit 0; }}\nprintf 'ready\\tfake\\t1\\n'\n\
             while IFS= read -r line; do echo \"$line\" >> '{}'; case \"$line\" in\n\
             listen*) printf 'level\\t0.6\\npartial\\tset extrude one depth\\n';;\n\
             stop) printf 'final\\t87\\tSet extrude one depth to twelve point five millimetres.\\n';;\n\
             quit) exit 0;;\nesac; done\n",
            log.display()
        ),
    )
    .unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
    p
}

fn open(name: &str) -> Option<(Headless, PathBuf)> {
    let mut h = Headless::new((1400.0, 900.0), 1.0).ok()?;
    let dir = scratch(name);
    let file = dir.join("plate.fg");
    std::fs::write(&file, PART).unwrap();
    h.gui.speech.exe = Some(fake(&dir));
    h.gui.speech.wav = Some(dir.join("said.wav"));
    // a message sent goes nowhere: no agent runs
    h.gui.agent.program = Some(dir.join("no-claude").display().to_string());
    h.gui.agent.settings_file = Some(dir.join("agent.toml"));
    h.step();
    h.gui.open(file.to_str().unwrap());
    h.settle();
    assert!(h.click("side:tab:Agent"));
    h.step();
    Some((h, dir))
}

#[test]
fn the_mic_button_listens_and_what_is_heard_lands_in_the_box_as_written() {
    let Some((mut h, dir)) = open("button") else {
        return;
    };
    assert!(wait(&mut h, |g| g.shell.agent.voice.unavailable.is_none()
        && g.shell.place("agent:mic").is_some()));
    assert!(h.click("agent:mic"));
    assert!(wait(&mut h, |g| g.shell.agent.voice.state == VoiceState::Listening));
    assert!(
        h.gui.shell.place("status:voice").is_some(),
        "the status bar says it's listening"
    );
    assert!(h.gui.shell.place("agent:mic:level").is_some(), "the meter");
    assert!(
        wait(&mut h, |g| g.shell.agent.input == "set extrude1 depth"),
        "{}",
        h.gui.shell.agent.input
    );
    assert!(h.click("agent:mic"));
    assert!(wait(&mut h, |g| g.shell.agent.voice.state == VoiceState::Idle));
    assert_eq!(h.gui.shell.agent.input, "Set extrude1 depth to 12.5 mm.");
    assert!(h.gui.shell.place("status:voice").is_none());
    assert!(
        h.gui.shell.agent.entries.is_empty(),
        "left in the box to edit, not sent"
    );
    let said = std::fs::read_to_string(dir.join("said.txt")).unwrap();
    let lines: Vec<&str> = said.lines().collect();
    assert!(
        lines[0].starts_with("names\t") && lines[0].contains("\textrude1"),
        "{said}"
    );
    assert!(
        lines[1].starts_with("listen-wav\t") && lines[1].ends_with("said.wav"),
        "{said}"
    );
    assert_eq!(lines[2], "stop");
    drop(h);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn the_key_held_talks_and_with_send_when_done_it_is_sent() {
    let Some((mut h, dir)) = open("key") else { return };
    let key = Chord::parse("F8").unwrap();
    let mut s = forge_render::shell::agent_settings::AgentSettings::from_extra(&h.gui.shell.settings.extra);
    s.speech_key = Some(key);
    s.speech_send = true;
    s.write_extra(&mut h.gui.shell.settings.extra);
    h.step();
    assert!(wait(&mut h, |g| g.shell.agent.voice.unavailable.is_none()));
    h.gui.shell.agent.input = "and then".into();
    h.key_event(key.key, Default::default(), true);
    assert!(wait(&mut h, |g| g.shell.agent.voice.state == VoiceState::Listening));
    assert!(wait(&mut h, |g| g.shell.agent.input == "and then set extrude1 depth"));
    h.key_event(key.key, Default::default(), false);
    assert!(wait(&mut h, |g| !g.shell.agent.entries.is_empty()));
    assert!(matches!(
        &h.gui.shell.agent.entries[0],
        forge_render::shell::agent::Entry::User { text, .. } if text == "and then Set extrude1 depth to 12.5 mm."
    ));
    // toggle: a press starts, the next stops; Esc gives up what was heard
    let mut s = forge_render::shell::agent_settings::AgentSettings::from_extra(&h.gui.shell.settings.extra);
    s.speech_mode = KeyMode::Toggle;
    s.speech_send = false;
    s.write_extra(&mut h.gui.shell.settings.extra);
    h.gui.shell.agent.running = false;
    h.gui.shell.agent.trust_ask = None;
    h.gui.shell.agent.input.clear();
    h.step();
    h.key(key.key);
    assert!(wait(&mut h, |g| g.shell.agent.input == "set extrude1 depth"));
    h.key(egui::Key::Escape);
    assert!(wait(&mut h, |g| g.shell.agent.voice.state == VoiceState::Idle));
    assert_eq!(h.gui.shell.agent.input, "");
    drop(h);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_shortcut_dictates_without_an_open_part_and_new_chat_cancels_it() {
    let Ok(mut h) = Headless::new((1400.0, 900.0), 1.0) else {
        return;
    };
    let dir = scratch("empty");
    h.gui.speech.exe = Some(fake(&dir));
    h.gui.speech.wav = Some(dir.join("said.wav"));
    let key = Chord::parse("F8").unwrap();
    let settings = forge_render::shell::agent_settings::AgentSettings {
        speech_key: Some(key),
        ..Default::default()
    };
    settings.write_extra(&mut h.gui.shell.settings.extra);
    h.step();
    assert!(wait(&mut h, |g| g.speech_checked()));
    h.key_event(key.key, Default::default(), true);
    assert!(wait(&mut h, |g| !g.shell.agent.input.is_empty()));
    assert_eq!(h.gui.shell.agent.voice.state, VoiceState::Listening);
    h.gui.shell.agent.new_chat();
    h.step();
    assert_eq!(h.gui.shell.agent.voice.state, VoiceState::Idle);
    assert_eq!(h.gui.shell.agent.input, "");
    drop(h);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn without_the_recogniser_the_button_says_speech_is_not_available() {
    let Ok(mut h) = Headless::new((1400.0, 900.0), 1.0) else {
        return;
    };
    let dir = scratch("none");
    let file = dir.join("plate.fg");
    std::fs::write(&file, PART).unwrap();
    h.gui.speech.exe = Some(dir.join("missing"));
    h.step();
    h.gui.open(file.to_str().unwrap());
    h.settle();
    assert!(h.click("side:tab:Agent"));
    assert!(wait(&mut h, |g| g.shell.agent.voice.unavailable.is_some()));
    assert!(h.click("agent:mic"));
    h.step();
    assert_eq!(h.gui.shell.agent.voice.state, VoiceState::Idle);
    drop(h);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
#[ignore = "needs forge-speech, its model and a WAV: FORGECAD_SPEECH, FORGECAD_SPEECH_TEST_WAV, FORGECAD_SPEECH_TEST_SAYS"]
fn a_wav_heard_by_forge_speech_lands_in_the_box() {
    let exe = PathBuf::from(std::env::var_os("FORGECAD_SPEECH").expect("FORGECAD_SPEECH"));
    let wav = PathBuf::from(std::env::var_os("FORGECAD_SPEECH_TEST_WAV").expect("FORGECAD_SPEECH_TEST_WAV"));
    let says = std::env::var("FORGECAD_SPEECH_TEST_SAYS").expect("FORGECAD_SPEECH_TEST_SAYS");
    let secs = std::fs::metadata(&wav).unwrap().len() as f64 / 32000.0;
    let mut h = Headless::new((1400.0, 900.0), 1.0).unwrap();
    let dir = scratch("real");
    let file = dir.join("plate.fg");
    std::fs::write(&file, PART).unwrap();
    h.gui.speech.exe = Some(exe);
    h.gui.speech.wav = Some(wav);
    h.step();
    h.gui.open(file.to_str().unwrap());
    h.settle();
    assert!(h.click("side:tab:Agent"));
    assert!(wait(&mut h, |g| g.shell.agent.voice.unavailable.is_none()
        && g.speech_checked()));
    assert!(h.click("agent:mic"));
    assert!(wait(&mut h, |g| g.shell.agent.voice.state == VoiceState::Listening));
    // the view turning all the while, each frame timed: it has to keep 60 fps while the model works
    let mut listening = Vec::new();
    let t = std::time::Instant::now();
    let mut partials = 0;
    let mut last = String::new();
    while t.elapsed().as_secs_f64() < secs + 0.3 {
        h.gui.turn_view();
        let f = std::time::Instant::now();
        h.step();
        listening.push(f.elapsed().as_secs_f64() * 1e3);
        if h.gui.shell.agent.input != last {
            last = h.gui.shell.agent.input.clone();
            partials += 1;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(h.click("agent:mic"));
    assert!(wait(&mut h, |g| g.shell.agent.voice.state == VoiceState::Idle));
    let ms = h.gui.shell.agent.voice.last_ms.unwrap();
    let mut warm = Vec::new();
    for _ in 0..300 {
        h.gui.turn_view();
        let f = std::time::Instant::now();
        h.step();
        warm.push(f.elapsed().as_secs_f64() * 1e3);
    }
    let q = |v: &mut Vec<f64>| {
        v.sort_by(f64::total_cmp);
        let at = |p: f64| v[((v.len() - 1) as f64 * p) as usize];
        format!(
            "p50 {:.1} p95 {:.1} max {:.1} ms over {}",
            at(0.5),
            at(0.95),
            at(1.0),
            v.len()
        )
    };
    eprintln!(
        "frames while listening: {}; with the model warm and idle: {}",
        q(&mut listening),
        q(&mut warm)
    );
    eprintln!(
        "heard {:?} ({partials} partials), {ms:.0} ms after stop, {:?}",
        h.gui.shell.agent.input, h.gui.shell.agent.voice.engine
    );
    assert!(partials >= 1);
    assert_eq!(h.gui.shell.agent.input, says);
    drop(h);
    let _ = std::fs::remove_dir_all(dir);
}

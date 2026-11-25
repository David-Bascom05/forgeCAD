//! Slash commands in the Agent tab, as in a terminal: `/` at the start of the box lists the
//! app's own (new, clear, resume, mode, model, undo, help) and then the backend's (Claude Code
//! says its own in `init`: compact, context, the person's skills); Tab writes one out, Enter
//! runs it. The app's run here; the backend's go to it as the message.
#![cfg(unix)]

use forge_render::agent::Event;
use forge_render::shell::agent::{AgentAct, AgentTab, Entry, Mode, command_candidates, slash_word};
use std::path::{Path, PathBuf};

#[test]
fn a_command_is_the_boxs_first_word_after_a_slash() {
    assert_eq!(slash_word("/com"), Some("com"));
    assert_eq!(slash_word("/"), Some(""));
    assert_eq!(
        slash_word("/anthropic-skills:docx"),
        Some("anthropic-skills:docx")
    );
    assert_eq!(
        slash_word("/mode plan"),
        None,
        "with its argument, it's written out"
    );
    assert_eq!(slash_word("make it /thicker"), None);
    let all = command_candidates(&[
        "compact".into(),
        "clear".into(),
        "__internal".into(),
        "me:skill".into(),
    ]);
    let names: Vec<&str> = all.iter().map(|c| c.name.as_str()).collect();
    // the app's first; the backend's own after, less the app's and the hidden
    assert_eq!(
        &names[..7],
        ["new", "clear", "resume", "mode", "model", "undo", "help"]
    );
    assert_eq!(&names[7..], ["compact", "me:skill"]);
    assert_eq!(all[7].what, "Summarise the conversation so far, to free context");
    assert_eq!((all[8].what.as_str(), all[8].source), ("A skill", "skill"));
}

fn run(tab: &mut AgentTab, text: &str) {
    tab.input = text.into();
    tab.send();
}

#[test]
fn the_apps_own_commands_run_here_and_never_reach_the_model() {
    let mut tab = AgentTab::default();
    run(&mut tab, "/mode plan");
    assert_eq!(tab.mode, Mode::Plan);
    assert_eq!(tab.out.pop(), Some(AgentAct::Mode(Mode::Plan)));
    run(&mut tab, "/mode");
    assert!(tab.mode_menu);
    run(&mut tab, "/mode sideways");
    assert!(matches!(
        tab.entries.last(),
        Some(Entry::Note { error: true, .. })
    ));
    run(&mut tab, "/model");
    assert!(tab.switcher);
    tab.commands = vec!["compact".into(), "me:skill".into(), "mystery".into()];
    run(&mut tab, "/help");
    let Some(Entry::Note { text, .. }) = tab.entries.last() else {
        panic!()
    };
    // one row a command, and whose when it isn't the app's
    let rows: Vec<&str> = text.lines().collect();
    assert!(
        rows.contains(&"/undo · Take back the last turn's changes"),
        "{text}"
    );
    assert!(rows.contains(&"/me:skill · A skill · skill"), "{text}");
    assert!(rows.contains(&"/mystery · Claude Code"), "{text}");
    run(&mut tab, "/undo");
    assert!(
        matches!(tab.entries.last(), Some(Entry::Note { text, .. }) if text.starts_with("Nothing to undo"))
    );
    // the last turn's two files, each taken back; an earlier turn's left as it is
    for (turn, index) in [(0, 0), (1, 0), (1, 1)] {
        tab.entries.push(Entry::Diff {
            turn,
            index,
            created: false,
            file: "plate.fg".into(),
            lines: vec![],
            added: 1,
            removed: 1,
            undone: false,
        });
    }
    tab.out.clear();
    run(&mut tab, "/undo");
    assert_eq!(tab.out, [AgentAct::Undo(1, 0), AgentAct::Undo(1, 1)]);
    run(&mut tab, "/clear");
    assert_eq!(tab.out.last(), Some(&AgentAct::NewChat));
    // where the last one went: said in the empty chat, not kept in it
    assert!(tab.entries.is_empty());
    assert!(tab.started_new);
    assert!(
        !tab.entries.iter().any(|e| matches!(e, Entry::User { .. })),
        "none was sent"
    );
    run(&mut tab, "thicker");
    assert!(!tab.started_new, "gone once there's a chat");
}

#[test]
fn claude_codes_init_says_its_commands() {
    let mut p = forge_render::agent::claude::Parser::default();
    let evs = p.line(r#"{"type":"system","subtype":"init","session_id":"s","model":"m","slash_commands":["compact","context","me:skill"]}"#);
    assert!(evs.contains(&Event::Commands(vec![
        "compact".into(),
        "context".into(),
        "me:skill".into()
    ])));
    let mut tab = AgentTab::default();
    for e in evs {
        tab.take(e);
    }
    assert_eq!(tab.commands, ["compact", "context", "me:skill"]);
}

/// A stand-in `claude` whose init names its commands, keeping each message it's sent.
fn fake(dir: &Path) -> PathBuf {
    let init = r#"{"type":"system","subtype":"init","session_id":"s","model":"m","slash_commands":["compact","context","me:skill"]}"#;
    let end = r#"{"type":"result","subtype":"success","session_id":"s"}"#;
    let script = dir.join("claude");
    std::fs::write(
        &script,
        format!(
            "#!/bin/sh\necho '{init}'\nwhile read line; do\n  echo \"$line\" >> '{d}/said.jsonl'\n  echo '{end}'\ndone\n",
            d = dir.display()
        ),
    )
    .unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    script
}

#[test]
fn in_the_app_slash_lists_and_enter_sends_claude_codes_own_command() {
    let Ok(mut h) = forge_render::gui::Headless::new((1400.0, 860.0), 1.0) else {
        return;
    };
    let dir = std::env::temp_dir().join(format!("forgecad-commands-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("plate.fg"), "param a.d = 1\n").unwrap();
    h.gui.agent.program = Some(fake(&dir).display().to_string());
    h.gui.agent.cwd = Some(dir.join("cwd"));
    h.gui.agent.settings_file = Some(dir.join("agent.toml"));
    h.gui.agent.conversations_dir = Some(dir.join("store"));
    h.step();
    h.gui.open(dir.join("plate.fg").to_str().unwrap());
    h.settle();
    assert!(h.click("side:tab:Agent"));
    // a first message, so Claude Code has said its commands
    assert!(h.click("agent:input"));
    h.text("hello");
    h.key(egui::Key::Enter);
    let t = std::time::Instant::now();
    while t.elapsed().as_secs() < 30 && (h.gui.shell.agent.commands.is_empty() || h.gui.shell.agent.running) {
        h.step();
    }
    assert_eq!(h.gui.shell.agent.commands, ["compact", "context", "me:skill"]);
    h.text("/com");
    h.step();
    h.step();
    assert!(
        h.gui.shell.place("agent:mention:compact").is_some(),
        "the list, Claude Code's too"
    );
    if let Ok(shots) = std::env::var("FORGE_UI_SHOTS") {
        for _ in 0..12 {
            h.step();
        }
        std::fs::create_dir_all(&shots).unwrap();
        let png = h.png().unwrap();
        std::fs::write(Path::new(&shots).join("commands-1400-dark-slash-list.png"), png).unwrap();
    }
    h.key(egui::Key::Enter);
    let t = std::time::Instant::now();
    while t.elapsed().as_secs() < 30
        && !std::fs::read_to_string(dir.join("said.jsonl"))
            .unwrap_or_default()
            .contains("/compact")
    {
        h.step();
    }
    let said = std::fs::read_to_string(dir.join("said.jsonl")).unwrap();
    assert!(said.contains(r#""content":"/compact""#), "{said}");
    // an app command typed and run with Enter: it doesn't go to the model
    h.text("/mo");
    h.step();
    h.step();
    h.key(egui::Key::ArrowDown);
    h.step();
    // "mode" then "model": the second
    h.key(egui::Key::Enter);
    h.step();
    assert!(h.gui.shell.agent.switcher, "/model opened the agent's switcher");
    assert!(
        !std::fs::read_to_string(dir.join("said.jsonl"))
            .unwrap()
            .contains("/model")
    );
    let _ = std::fs::remove_dir_all(dir);
}

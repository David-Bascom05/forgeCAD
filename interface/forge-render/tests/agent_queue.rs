//! Messages sent while the agent works wait their turn; each waiting one can be taken back into
//! the box to change, or dropped, and then never reaches the agent.
// a fake agent CLI as a shell script: Unix only
#![cfg(unix)]

use forge_render::shell::agent::Entry;
use forge_render::shell::agent_settings::{AgentSettings, Kind};
use std::path::{Path, PathBuf};

/// A stand-in `claude`: each message goes to `heard.jsonl`, and its turn lasts until `go` exists.
fn fake(dir: &Path) -> PathBuf {
    let init = r#"{"type":"system","subtype":"init","session_id":"s","model":"m"}"#;
    let end = r#"{"type":"result","subtype":"success","session_id":"s"}"#;
    let script = dir.join("claude");
    std::fs::write(
        &script,
        format!(
            "#!/bin/sh\necho '{init}'\nwhile read line; do\n  echo \"$line\" >> '{d}/heard.jsonl'\n  while [ ! -e '{d}/go' ]; do sleep 0.02; done\n  rm -f '{d}/go'\n  echo '{end}'\ndone\n",
            d = dir.display()
        ),
    )
    .unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    script
}

fn wait(h: &mut forge_render::gui::Headless, what: impl Fn(&forge_render::gui::Gui) -> bool) -> bool {
    let t = std::time::Instant::now();
    while t.elapsed().as_secs() < 30 {
        h.step();
        if what(&h.gui) {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    false
}

fn users(g: &forge_render::gui::Gui) -> Vec<String> {
    g.shell
        .agent
        .entries
        .iter()
        .filter_map(|e| match e {
            Entry::User { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

fn say(h: &mut forge_render::gui::Headless, text: &str) {
    assert!(h.click("agent:input"));
    h.text(text);
    assert!(h.click("agent:send"));
    h.step();
}

#[test]
fn a_queued_message_can_be_dropped_or_taken_back_to_edit() {
    let dir = std::env::temp_dir().join(format!("forgecad-queue-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let dir = std::fs::canonicalize(dir).unwrap();
    std::fs::write(dir.join("plate.fg"), "param d = 12\n").unwrap();
    let Ok(mut h) = forge_render::gui::Headless::new((1400.0, 860.0), 1.0) else {
        return;
    };
    h.gui.agent.program = Some(fake(&dir).display().to_string());
    h.gui.agent.cwd = Some(dir.clone());
    h.gui.agent.settings_file = Some(dir.join("agent.toml"));
    h.step();
    AgentSettings {
        kind: Kind::ClaudeCode,
        ..Default::default()
    }
    .write_extra(&mut h.gui.shell.settings.extra);
    h.gui.open(dir.join("plate.fg").to_str().unwrap());
    h.settle();
    assert!(h.click("side:tab:Agent"));
    say(&mut h, "first");
    assert!(wait(&mut h, |_| dir.join("heard.jsonl").exists()));
    say(&mut h, "second");
    say(&mut h, "third");
    assert!(wait(&mut h, |g| g.shell.agent.queue == ["second", "third"]));
    // waiting, they're their queue rows only: no card says they were sent
    assert_eq!(users(&h.gui), ["first"]);
    assert!(
        h.gui.shell.place("agent:queued:1:drop").is_some(),
        "each queued, with its ×"
    );
    if let Ok(shots) = std::env::var("FORGE_UI_SHOTS") {
        for _ in 0..6 {
            h.step();
        }
        std::fs::create_dir_all(&shots).unwrap();
        std::fs::write(
            Path::new(&shots).join("queue-1400-dark-two-queued.png"),
            h.png().unwrap(),
        )
        .unwrap();
    }
    // dropped: gone from the queue and the chat
    assert!(h.click("agent:queued:1:drop"));
    // the chat settles at its end again, a card shorter
    for _ in 0..10 {
        h.step();
    }
    assert_eq!(h.gui.shell.agent.queue, ["second"]);
    assert_eq!(users(&h.gui), ["first"]);
    // taken back: in the box to change, out of the queue and the chat
    assert!(h.click("agent:queued:0:edit"));
    h.step();
    assert!(
        h.gui.shell.agent.queue.is_empty(),
        "{:?} input {:?} places {:?}",
        h.gui.shell.agent.queue,
        h.gui.shell.agent.input,
        h.gui
            .shell
            .places
            .iter()
            .filter(|p| p.0.starts_with("agent:queued"))
            .collect::<Vec<_>>()
    );
    assert_eq!(h.gui.shell.agent.queued, 0);
    assert_eq!(h.gui.shell.agent.input, "second");
    assert_eq!(users(&h.gui), ["first"]);
    // the turn ends: neither goes
    std::fs::write(dir.join("go"), "").unwrap();
    assert!(wait(&mut h, |g| !g.shell.agent.running));
    for _ in 0..20 {
        h.step();
    }
    let heard = std::fs::read_to_string(dir.join("heard.jsonl")).unwrap();
    assert_eq!(
        heard.lines().count(),
        1,
        "only the first reached the agent: {heard}"
    );
    // changed and sent again, it goes
    h.gui.shell.agent.input = "second, changed".into();
    assert!(h.click("agent:send"));
    assert!(wait(&mut h, |_| std::fs::read_to_string(dir.join("heard.jsonl"))
        .unwrap()
        .contains("second, changed")));
    // one more behind it: its card comes when it goes, after the turn before it
    say(&mut h, "fourth");
    assert!(wait(&mut h, |g| g.shell.agent.queue == ["fourth"]));
    assert_eq!(users(&h.gui), ["first", "second, changed"]);
    std::fs::write(dir.join("go"), "").unwrap();
    assert!(wait(&mut h, |_| std::fs::read_to_string(dir.join("heard.jsonl"))
        .unwrap()
        .contains("fourth")));
    assert_eq!(users(&h.gui), ["first", "second, changed", "fourth"]);
    assert!(h.gui.shell.agent.queue.is_empty());
    std::fs::write(dir.join("go"), "").unwrap();
    let _ = std::fs::remove_dir_all(dir);
}

//! The Agent tab, nearer a terminal: what a turn took (its time so far while it runs, then its
//! time and tokens), the plan's limit said, and ↑/↓ bringing back the messages sent before.
#![cfg(unix)]

use forge_render::agent::Event;
use forge_render::agent::claude::Parser;
use forge_render::shell::agent::{duration, thousands};
use std::path::{Path, PathBuf};

#[test]
fn times_and_counts_read_as_a_person_says_them() {
    assert_eq!(duration(8.4), "8 s");
    assert_eq!(duration(125.0), "2 min 5 s");
    assert_eq!(thousands(340), "340");
    assert_eq!(thousands(21_300), "21.3k");
    assert_eq!(thousands(1_250_000), "1.2M");
}

#[test]
fn a_turns_end_says_what_it_took_and_a_limit_is_said() {
    let mut p = Parser::default();
    let evs = p.line(r#"{"type":"result","subtype":"success","duration_ms":14200,"total_cost_usd":0.052,"usage":{"input_tokens":120,"cache_read_input_tokens":21000,"cache_creation_input_tokens":180,"output_tokens":340},"session_id":"s"}"#);
    assert_eq!(
        evs[0],
        Event::Usage {
            secs: 14.2,
            tokens_in: 21_300,
            tokens_out: 340,
            cost: 0.052
        }
    );
    assert!(matches!(
        evs[1],
        Event::Done {
            error: None,
            stopped: false
        }
    ));
    // allowed: nothing said; reached: said, with when it resets
    assert_eq!(
        p.line(r#"{"type":"rate_limit_event","rate_limit_info":{"status":"allowed","resetsAt":0}}"#),
        [Event::Limit {
            reached: false,
            close: false,
            resets: Some(0.0)
        }]
    );
    let evs =
        p.line(r#"{"type":"rate_limit_event","rate_limit_info":{"status":"rejected","resetsAt":5000}}"#);
    assert_eq!(
        evs,
        [Event::Limit {
            reached: true,
            close: false,
            resets: Some(5000.0)
        }]
    );
    let evs = p.line(r#"{"type":"rate_limit_event","rate_limit_info":{"status":"allowed_warning"}}"#);
    assert_eq!(
        evs,
        [Event::Limit {
            reached: false,
            close: true,
            resets: None
        }]
    );
    // in words, said under the last turn: when it resets, under a day in hours and minutes
    use forge_render::shell::agent::limit_words;
    assert_eq!(
        limit_words(true, Some(1000.0 + 3.0 * 3600.0 + 1200.0), 1000.0),
        "Plan limit reached · resets in 3 h 20 min"
    );
    assert_eq!(
        limit_words(true, Some(1000.0 + 7200.0), 1000.0),
        "Plan limit reached · resets in 2 h"
    );
    assert_eq!(
        limit_words(false, Some(1000.0 + 600.0), 1000.0),
        "Close to your plan's limit · resets in 10 min"
    );
    assert_eq!(
        limit_words(false, Some(1000.0 + 3.0 * 86400.0), 1000.0),
        "Close to your plan's limit · resets in 3 days"
    );
    assert_eq!(limit_words(true, None, 0.0), "Plan limit reached");
    // the tab: amber when reached, gone when allowed again
    let mut tab = forge_render::shell::agent::AgentTab::default();
    tab.take(Event::Limit {
        reached: true,
        close: false,
        resets: None,
    });
    assert_eq!(tab.limit, Some((true, None)));
    tab.take(Event::Limit {
        reached: false,
        close: false,
        resets: None,
    });
    assert_eq!(tab.limit, None);
    // a turn's cost only when paid per token
    assert_eq!(
        p.line(r#"{"type":"system","subtype":"init","session_id":"s","model":"m","apiKeySource":"none"}"#)
            .last(),
        Some(&Event::ApiKeyed(false))
    );
    assert_eq!(
        p.line(r#"{"type":"system","subtype":"init","session_id":"s","model":"m","apiKeySource":"ANTHROPIC_API_KEY"}"#).last(),
        Some(&Event::ApiKeyed(true))
    );
}

/// A stand-in `claude`: each message, a turn that took 2 s and some tokens.
fn fake(dir: &Path) -> PathBuf {
    let init = r#"{"type":"system","subtype":"init","session_id":"s","model":"m"}"#;
    let text = r#"{"type":"assistant","message":{"id":"m-1","content":[{"type":"text","text":"Done."}]},"parent_tool_use_id":null}"#;
    let end = r#"{"type":"result","subtype":"success","duration_ms":2000,"total_cost_usd":0.01,"usage":{"input_tokens":1500,"output_tokens":20},"session_id":"s"}"#;
    let script = dir.join("claude");
    std::fs::write(
        &script,
        format!("#!/bin/sh\necho '{init}'\nwhile read line; do\n  echo '{text}'\n  echo '{end}'\ndone\n"),
    )
    .unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    script
}

fn say(h: &mut forge_render::gui::Headless, text: &str) {
    h.text(text);
    h.key(egui::Key::Enter);
    let t = std::time::Instant::now();
    h.step();
    while t.elapsed().as_secs() < 30 && h.gui.shell.agent.running {
        h.step();
    }
}

#[test]
fn up_brings_back_what_was_sent_and_the_turn_says_what_it_took() {
    let Ok(mut h) = forge_render::gui::Headless::new((1400.0, 860.0), 1.0) else {
        return;
    };
    let dir = std::env::temp_dir().join(format!("forgecad-polish-{}", std::process::id()));
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
    assert!(h.click("agent:input"));
    say(&mut h, "first ask");
    say(&mut h, "second ask");
    for _ in 0..3 {
        h.step();
    }
    // the last turn's time and tokens, under it
    assert_eq!(h.gui.shell.agent.usage.map(|u| (u.1, u.2)), Some((1500, 20)));
    assert!(h.gui.shell.place("agent:usage").is_some());
    if let Ok(shots) = std::env::var("FORGE_UI_SHOTS") {
        // a reply with Markdown, for the picture
        h.gui.shell.agent.entries.push(forge_render::shell::agent::Entry::Agent {
            text: "## What changed\n- **extrude1** depth from 12 to 14 mm\n- `fillet1` kept at 2 mm\n> The part builds: 0 errors.\n\n| feature | value | was | drives |\n|---|--:|--:|---|\n| `extrude1.d` | 14 mm | 12 mm | the plate's thickness, and the holes' depth |\n| `fillet1.r` | 2 mm | 2 mm | the top edges |".into(),
        });
        h.gui.shell.agent.limit = Some((true, Some(forge_history::now() as f64 + 3.0 * 3600.0 + 1200.0)));
        for _ in 0..12 {
            h.step();
        }
        std::fs::create_dir_all(&shots).unwrap();
        let png = h.png().unwrap();
        std::fs::write(Path::new(&shots).join("polish2-1400-dark-markdown-usage.png"), png).unwrap();
        h.gui.shell.agent.entries.pop();
        h.gui.shell.agent.limit = None;
    }
    // ↑ ↑ ↓ ↓ in the empty box
    let input = |h: &forge_render::gui::Headless| h.gui.shell.agent.input.clone();
    for (key, want) in [
        (egui::Key::ArrowUp, "second ask"),
        (egui::Key::ArrowUp, "first ask"),
        (egui::Key::ArrowUp, "first ask"),
        (egui::Key::ArrowDown, "second ask"),
        (egui::Key::ArrowDown, ""),
    ] {
        h.key(key);
        h.step();
        assert_eq!(input(&h), want, "{key:?}");
    }
    // typed over: the arrows are the cursor's again
    h.key(egui::Key::ArrowUp);
    h.step();
    h.text(" more");
    h.step();
    h.key(egui::Key::ArrowUp);
    h.step();
    assert_eq!(input(&h), "second ask more");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_long_tool_output_shows_all_of_itself_when_asked() {
    use forge_render::shell::agent::{Entry, ToolState};
    let Ok(mut h) = forge_render::gui::Headless::new((1400.0, 860.0), 1.0) else {
        return;
    };
    h.step();
    h.gui.shell.agent.tab = forge_render::shell::agent::SideTab::Agent;
    let output: String = (0..64).map(|k| format!("line {k}\n")).collect();
    h.gui.shell.agent.restore_entries(vec![Entry::Tool {
        id: "t".into(),
        name: "Bash".into(),
        summary: "Ran ls".into(),
        input: String::new(),
        output,
        state: ToolState::Done,
    }]);
    h.step();
    assert!(h.click("agent:tool:t"));
    // opened at the frame's end: drawn open the next
    h.step();
    h.step();
    assert!(
        h.gui.shell.place("agent:tool-all:t").is_some(),
        "Show all 64 lines"
    );
    // down to it, as a reader scrolls
    let row = h.gui.shell.place("agent:tool:t").unwrap();
    h.move_to(row.center().x, 400.0);
    for _ in 0..20 {
        h.wheel(-400.0);
        h.step();
    }
    assert!(h.click("agent:tool-all:t"));
    h.step();
    assert!(h.gui.shell.agent.open.contains("all:t"));
    h.step();
    // and back (it's lower now: down to it again)
    for _ in 0..20 {
        h.wheel(-400.0);
        h.step();
    }
    assert!(h.click("agent:tool-all:t"));
    h.step();
    assert!(!h.gui.shell.agent.open.contains("all:t"));
}

/// The audit's: "+" on an empty chat did nothing and said nothing. Now it's off, and says why;
/// with a chat, it starts a new one.
#[test]
fn new_chat_is_off_on_an_empty_chat_and_starts_one_otherwise() {
    use forge_render::shell::agent::Entry;
    let Ok(mut h) = forge_render::gui::Headless::new((1400.0, 860.0), 1.0) else {
        return;
    };
    h.step();
    assert!(h.click("side:tab:Agent"));
    h.step();
    assert!(h.click("agent:new"));
    h.step();
    assert!(h.gui.shell.agent.entries.is_empty());
    assert!(h.gui.shell.agent.out.is_empty(), "nothing asked of the host");
    h.gui.shell.agent.restore_entries(vec![Entry::Note {
        text: "an earlier chat".into(),
        error: false,
    }]);
    h.step();
    assert!(h.click("agent:new"));
    h.step();
    assert!(h.gui.shell.agent.entries.is_empty(), "a new chat");
}

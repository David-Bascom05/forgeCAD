//! The Agent tab asks before a tool runs, as a terminal does: Claude Code started with
//! `--permission-prompt-tool stdio` sends `can_use_tool` control requests (tests/fixtures/agent/
//! permission.jsonl, captured from 2.1.331 with paths trimmed), the tab shows each as a card,
//! and the answer goes back as a `control_response`. The permission mode is the header's chip
//! and Shift+Tab. The project watch reads only files whose size or time changed.
// a fake agent CLI as a shell script: Unix only
#![cfg(unix)]

use forge_agent::json::Json;
use forge_render::agent::claude::{Parser, always_words};
use forge_render::agent::{Event, Out, Watch};
use forge_render::shell::agent::{AgentAct, AgentTab, Answer, Asked, Entry, Mode};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::channel;

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

fn fixture(name: &str) -> String {
    std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/agent")
            .join(name),
    )
    .unwrap()
}

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("forgecad-approvals-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn wait(h: &mut forge_render::gui::Headless, what: impl Fn(&forge_render::gui::Gui) -> bool) -> bool {
    let t = std::time::Instant::now();
    while t.elapsed().as_secs() < 120 {
        h.step();
        if what(&h.gui) {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    false
}

#[test]
fn a_permission_request_reads_as_a_card_with_what_always_would_allow() {
    let mut p = Parser::default();
    let evs: Vec<Event> = fixture("permission.jsonl")
        .lines()
        .flat_map(|l| p.line(l))
        .collect();
    let modes: Vec<&str> = evs
        .iter()
        .filter_map(|e| match e {
            Event::Mode(m) => Some(m.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(modes, ["default", "acceptEdits", "bypassPermissions"]);
    let asks: Vec<_> = evs
        .iter()
        .filter_map(|e| match e {
            Event::Permission {
                id,
                tool,
                tool_use,
                summary,
                input,
                always,
                ..
            } => {
                assert!(tool_use.starts_with("toolu_perm_"), "its tool call: {tool_use}");
                Some((
                    id.as_str(),
                    tool.as_str(),
                    summary.as_str(),
                    input.as_str(),
                    always.as_deref(),
                ))
            }
            _ => None,
        })
        .collect();
    assert_eq!(asks.len(), 2);
    let (id, tool, summary, input, always) = asks[0];
    assert_eq!((id, tool), ("8ce4796a-7788-4946-86e6-e8fa78c01b87", "Edit"));
    // asked in the present tense: what it would do
    assert!(
        summary.starts_with("Edit ") && summary.contains("plate.fg"),
        "{summary}"
    );
    assert!(
        input.contains("param extrude1.d = 12") && input.contains("param extrude1.d = 14"),
        "{input}"
    );
    assert_eq!(always, Some("Accept edits for this chat"));
    assert_eq!(
        asks[1].4,
        Some("Allow Bash(curl -sI https://example.com) in this project, on this computer")
    );
    // the request's input and suggestions are kept for the answer
    assert_eq!(p.asked.len(), 2);
    assert_eq!(
        p.asked[0].1.get("new_string").and_then(Json::str),
        Some("param extrude1.d = 14")
    );
    // a refusal by a rule is said, not just a red dot
    assert!(
        evs.iter()
            .any(|e| matches!(e, Event::Note { text, error: true } if text.contains("rm -rf build")))
    );
    assert_eq!(always_words(&Json::Arr(vec![])), None);
}

#[test]
fn the_tab_keeps_a_question_until_answered_and_a_turns_end_drops_it() {
    let mut tab = AgentTab::default();
    let mut p = Parser::default();
    for l in fixture("permission.jsonl").lines() {
        for e in p.line(l) {
            tab.take(e);
        }
    }
    assert_eq!(tab.waiting(), Some("8ce4796a-7788-4946-86e6-e8fa78c01b87"));
    // a mode the tab has no name for is shown as said
    assert_eq!(tab.mode_other.as_deref(), Some("bypassPermissions"));
    tab.answer("8ce4796a-7788-4946-86e6-e8fa78c01b87", Answer::Always);
    assert_eq!(
        tab.out,
        [AgentAct::Answer(
            "8ce4796a-7788-4946-86e6-e8fa78c01b87".into(),
            Answer::Always
        )]
    );
    // answered once only
    tab.answer("8ce4796a-7788-4946-86e6-e8fa78c01b87", Answer::Deny);
    assert_eq!(tab.out.len(), 1);
    assert_eq!(tab.waiting(), Some("0b6c8f1e-1111-4222-8333-444455556666"));
    tab.take(Event::Done {
        error: None,
        stopped: true,
    });
    assert_eq!(tab.waiting(), None);
    let states: Vec<Asked> = tab
        .entries
        .iter()
        .filter_map(|e| match e {
            Entry::Permission { state, .. } => Some(*state),
            _ => None,
        })
        .collect();
    assert_eq!(states, [Asked::Always, Asked::Dropped]);
    tab.set_mode(Mode::Plan);
    assert!(tab.mode_chosen && tab.mode_other.is_none());
    assert_eq!(tab.out.last(), Some(&AgentAct::Mode(Mode::Plan)));
}

#[test]
fn claude_code_asks_the_app_and_keeps_the_persons_own_mode_unless_they_choose() {
    use forge_render::agent::Backend;
    let dir = scratch("flags");
    let (tx, _rx) = channel();
    let mut c = forge_render::agent::claude::ClaudeCode::new(
        forge_render::agent::claude::Config {
            program: "claude".into(),
            model: String::new(),
            file: dir.join("p.fg"),
            system: "s".into(),
            mcp: None,
            cwd: dir.join("cwd"),
        },
        Out {
            tx,
            wake: Arc::new(|| {}),
        },
        None,
    );
    let has = |a: &[String], pair: &[&str]| {
        a.windows(pair.len())
            .any(|w| w.iter().zip(pair).all(|(x, y)| x == y))
    };
    let a = c.args();
    assert!(has(&a, &["--permission-prompt-tool", "stdio"]));
    assert!(
        !a.iter().any(|x| x == "--permission-mode"),
        "their settings' defaultMode stands"
    );
    c.set_mode(Mode::Plan);
    assert!(has(&c.args(), &["--permission-mode", "plan"]));
    let _ = std::fs::remove_dir_all(dir);
}

/// A stand-in `claude`: it asks to Edit the part, and writes every line the app sends after the
/// first message to `heard.jsonl`; it makes the edit only if the answer allowed it.
fn fake_claude(dir: &Path) -> PathBuf {
    let init = r#"{"type":"system","subtype":"init","session_id":"s-1","model":"claude-haiku-5-5","permissionMode":"default"}"#;
    let ask = r#"{"type":"control_request","request_id":"req-1","request":{"subtype":"can_use_tool","tool_name":"Edit","input":{"file_path":"/elsewhere/proj/plate.fg","old_string":"= 12","new_string":"= 14"},"permission_suggestions":[{"type":"setMode","mode":"acceptEdits","destination":"session"}],"tool_use_id":"t-1"}}"#;
    let tool = r#"{"type":"assistant","message":{"id":"m-1","content":[{"type":"tool_use","id":"t-1","name":"Edit","input":{"file_path":"plate.fg","old_string":"param extrude1.d = 12","new_string":"param extrude1.d = 14"}}]},"parent_tool_use_id":null}"#;
    let script = dir.join("claude");
    std::fs::write(
        &script,
        format!(
            "#!/bin/sh\nread line\necho '{init}'\necho '{tool}'\necho '{ask}'\nread reply\necho \"$reply\" >> '{d}/heard.jsonl'\ncase \"$reply\" in *'\"allow\"'*) sed -i 's/^param extrude1.d = 12$/param extrude1.d = 14/' '{d}/plate.fg'; echo '{{\"type\":\"user\",\"message\":{{\"content\":[{{\"type\":\"tool_result\",\"tool_use_id\":\"t-1\",\"content\":\"ok\"}}]}},\"parent_tool_use_id\":null}}';; *) echo '{{\"type\":\"user\",\"message\":{{\"content\":[{{\"type\":\"tool_result\",\"tool_use_id\":\"t-1\",\"content\":\"declined\",\"is_error\":true}}]}},\"parent_tool_use_id\":null}}';; esac\necho '{{\"type\":\"result\",\"subtype\":\"success\",\"session_id\":\"s-1\"}}'\nwhile read line; do echo \"$line\" >> '{d}/heard.jsonl'; done\n",
            d = dir.display()
        ),
    )
    .unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    script
}

fn heard(dir: &Path) -> Vec<Json> {
    std::fs::read_to_string(dir.join("heard.jsonl"))
        .unwrap_or_default()
        .lines()
        .filter_map(|l| Json::parse(l).ok())
        .collect()
}

fn app(name: &str) -> Option<(forge_render::gui::Headless, PathBuf)> {
    let mut h = forge_render::gui::Headless::new((1400.0, 860.0), 1.0).ok()?;
    let dir = scratch(name);
    std::fs::write(dir.join("plate.fg"), PART).unwrap();
    h.gui.agent.program = Some(fake_claude(&dir).display().to_string());
    h.gui.agent.cwd = Some(dir.join("cwd"));
    h.gui.agent.settings_file = Some(dir.join("agent.toml"));
    h.step();
    h.gui.open(dir.join("plate.fg").to_str().unwrap());
    h.settle();
    assert!(h.click("side:tab:Agent"));
    assert!(h.click("agent:input"));
    h.text("make the base plate thicker");
    assert!(h.click("agent:send"));
    assert!(wait(&mut h, |g| g.shell.agent.waiting().is_some()));
    Some((h, dir))
}

#[test]
fn allow_runs_the_edit_and_the_answer_carries_its_input() {
    let Some((mut h, dir)) = app("allow") else { return };
    // Claude Code sends only the snippet ("= 12"): the card shows it against the file, so the
    // line it's on, and the param's name, are in it
    let card = h.gui.shell.agent.entries.iter().find_map(|e| match e {
        Entry::Permission { input, .. } => Some(input.clone()),
        _ => None,
    });
    let card = card.unwrap();
    assert!(card.contains("- param extrude1.d = 12"), "{card}");
    assert!(card.contains("+ param extrude1.d = 14"), "{card}");
    // the question can't be missed from the Model tab either
    assert!(h.gui.shell.place("agent:perm:req-1:allow").is_some());
    assert!(h.gui.shell.place("agent:perm:req-1:always").is_some());
    assert!(h.click("agent:perm:req-1:allow"));
    assert!(wait(&mut h, |g| !g.shell.agent.running
        && g.shell
            .agent
            .entries
            .iter()
            .any(|e| matches!(e, Entry::Diff { .. }))));
    assert!(h.gui.fg.as_ref().unwrap().src.contains("param extrude1.d = 14"));
    let said = heard(&dir);
    let r = said[0].get("response").unwrap();
    assert_eq!(said[0].get("type").and_then(Json::str), Some("control_response"));
    assert_eq!(r.get("request_id").and_then(Json::str), Some("req-1"));
    let inner = r.get("response").unwrap();
    assert_eq!(inner.get("behavior").and_then(Json::str), Some("allow"));
    assert_eq!(
        inner
            .get("updatedInput")
            .and_then(|i| i.get("new_string"))
            .and_then(Json::str),
        Some("= 14"),
        "the input as Claude Code sent it, not the card's"
    );
    assert!(h.gui.shell.agent.entries.iter().any(|e| matches!(
        e,
        Entry::Permission {
            state: Asked::Allowed,
            ..
        }
    )));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn esc_declines_and_the_mode_chip_tells_the_running_agent() {
    let Some((mut h, dir)) = app("deny") else { return };
    h.key(egui::Key::Escape);
    assert!(wait(&mut h, |g| !g.shell.agent.running));
    assert!(
        h.gui.fg.as_ref().unwrap().src.contains("param extrude1.d = 12"),
        "nothing written"
    );
    assert!(h.gui.shell.agent.entries.iter().any(|e| matches!(
        e,
        Entry::Permission {
            state: Asked::Denied,
            ..
        }
    )));
    let said = heard(&dir);
    assert_eq!(
        said[0]
            .get("response")
            .and_then(|r| r.get("response"))
            .and_then(|r| r.get("behavior"))
            .and_then(Json::str),
        Some("deny")
    );
    // the header's chip: Ask first → Accept edits, sent to the process that's still there
    assert_eq!(h.gui.shell.agent.mode, Mode::Ask);
    assert!(h.click("agent:mode"));
    h.step();
    assert!(h.click("agent:mode:acceptEdits"));
    h.step();
    assert_eq!(h.gui.shell.agent.mode, Mode::AcceptEdits);
    let t = std::time::Instant::now();
    while t.elapsed().as_secs() < 30 && heard(&dir).len() < 2 {
        h.step();
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let said = heard(&dir);
    let req = said[1].get("request").unwrap();
    assert_eq!(
        req.get("subtype").and_then(Json::str),
        Some("set_permission_mode")
    );
    assert_eq!(req.get("mode").and_then(Json::str), Some("acceptEdits"));
    // Shift+Tab in the box goes on to Plan only
    assert!(h.click("agent:input"));
    h.key_with(egui::Key::Tab, egui::Modifiers::SHIFT);
    h.step();
    assert_eq!(h.gui.shell.agent.mode, Mode::Plan);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn the_watch_sees_a_same_size_edit_and_skips_what_did_not_change() {
    let dir = scratch("watch");
    std::fs::write(dir.join("a.fg"), "param d = 12\n").unwrap();
    std::fs::write(dir.join("b.fg"), "param e = 1\n").unwrap();
    let mut w = Watch::new(&dir);
    assert!(w.check().is_empty());
    // same length, at once: still seen (a time this fresh is read again)
    std::fs::write(dir.join("a.fg"), "param d = 14\n").unwrap();
    let got = w.check();
    assert_eq!(got.len(), 1);
    assert!(
        matches!(&got[0], Event::FileChanged { path, before: Some(b), text }
        if path.ends_with("a.fg") && b == "param d = 12\n" && text == "param d = 14\n")
    );
    std::fs::write(dir.join("c.fg"), "new\n").unwrap();
    let got = w.check();
    assert!(matches!(&got[..], [Event::FileChanged { before: None, .. }]));
    let _ = std::fs::remove_dir_all(dir);
}

/// UX review pictures: a long chat, a running tool and a question waiting, at 1400×860 and
/// 480 px, dark and light; then the answered card and the Model tab's badge.
/// `FORGE_UI_SHOTS=DIR` writes them.
#[test]
fn approvals_pictures() {
    use forge_render::shell::agent::ToolState;
    for (w, tag) in [(1400.0, "1400"), (480.0, "480")] {
        for dark in [true, false] {
            let Ok(mut h) = forge_render::gui::Headless::new((w, 860.0), 1.0) else {
                return;
            };
            let dir = scratch(&format!("shots-{tag}-{dark}"));
            std::fs::write(dir.join("plate.fg"), PART).unwrap();
            h.gui.agent.program = Some(fake_claude(&dir).display().to_string());
            h.gui.agent.cwd = Some(dir.join("cwd"));
            h.gui.agent.settings_file = Some(dir.join("agent.toml"));
            let mut s = h.gui.shell.settings.clone();
            s.dark = dark;
            h.gui.shell.use_settings(s);
            h.step();
            h.gui.open(dir.join("plate.fg").to_str().unwrap());
            h.settle();
            assert!(h.click("side:tab:Agent"));
            let mut long = Vec::new();
            for k in 0..12 {
                long.push(Entry::User {
                    text: format!("Round {k}: fillet the top edges 2 mm and check the wall"),
                    refs: vec![],
                    markup: None,
                });
                long.push(Entry::Tool {
                    id: format!("old{k}"),
                    name: "mcp__forgecad__fg_run".into(),
                    summary: "Built plate.fg".into(),
                    input: String::new(),
                    output: "0 errors, 0 warnings".into(),
                    state: ToolState::Done,
                });
                long.push(Entry::Agent {
                    text: "Added `fillet1` (2 mm) on the top edges; the part builds, 0 errors.".into(),
                });
            }
            h.gui.shell.agent.restore_entries(long);
            assert!(h.click("agent:input"));
            h.text("make the base plate thicker");
            assert!(h.click("agent:send"));
            assert!(wait(&mut h, |g| g.shell.agent.waiting().is_some()));
            // a tool still running beside the question
            let n = h.gui.shell.agent.entries.len();
            h.gui.shell.agent.entries.insert(
                n - 1,
                Entry::Tool {
                    id: "run".into(),
                    name: "mcp__forgecad__fg_query".into(),
                    summary: "Query plate.fg: holes".into(),
                    input: String::new(),
                    output: String::new(),
                    state: ToolState::Running,
                },
            );
            let theme = if dark { "dark" } else { "light" };
            shot(&mut h, &format!("approvals-{tag}-{theme}-waiting"));
            if tag == "1400" && dark {
                assert!(h.click("side:tab:Model"));
                shot(&mut h, "approvals-1400-dark-badge-on-model-tab");
                assert!(h.click("side:tab:Agent"));
                assert!(h.click("agent:perm:req-1:allow"));
                assert!(wait(&mut h, |g| !g.shell.agent.running));
                assert!(h.click("agent:mode"));
                shot(&mut h, "approvals-1400-dark-answered-accept-edits");
            }
            drop(h);
            let _ = std::fs::remove_dir_all(dir);
        }
    }
}

fn shot(h: &mut forge_render::gui::Headless, name: &str) {
    for _ in 0..12 {
        h.step();
    }
    if let Ok(dir) = std::env::var("FORGE_UI_SHOTS") {
        std::fs::create_dir_all(&dir).unwrap();
        if let Ok(png) = h.png() {
            std::fs::write(Path::new(&dir).join(format!("{name}.png")), png).unwrap();
        }
    }
}

#[test]
fn a_tool_row_says_what_is_true_now() {
    use forge_render::shell::agent::{not_done, present, progressive};
    // asking: what it would do; running: what it's doing; declined: what it didn't
    assert_eq!(present("Edited plate.fg"), "Edit plate.fg");
    assert_eq!(progressive("Edited plate.fg"), "Editing plate.fg");
    assert_eq!(progressive("Query plate.fg: holes"), "Querying plate.fg: holes");
    assert_eq!(not_done("Edited plate.fg"), "Not edited: plate.fg");
    assert_eq!(not_done("Ran cargo build"), "Not run: cargo build");
    // a summary that doesn't start with a known verb stays as it is
    assert_eq!(present("mcp__x__y"), "mcp__x__y");
    assert_eq!(progressive("Read"), "Reading");
}

#[test]
fn one_params_edit_is_said_in_words() {
    use forge_render::shell::agent::param_change;
    let edit = "- param extrude1.d = 12\n+ param extrude1.d = 14\n";
    assert_eq!(
        param_change(edit),
        Some(("extrude1.d".into(), "12".into(), "14".into()))
    );
    // two params, a non-param line changed, a param renamed: not one value's change
    assert_eq!(
        param_change("- param a = 1\n- param b = 2\n+ param a = 3\n+ param b = 4"),
        None
    );
    assert_eq!(
        param_change("- extrude1 = extrude(sk1, d)\n+ extrude1 = extrude(sk1, 2 * d)"),
        None
    );
    assert_eq!(param_change("- param a = 1\n+ param b = 1"), None);
}

#[test]
fn the_mode_chip_opens_its_menu_and_esc_closes_it() {
    let Some((mut h, dir)) = app("menu") else { return };
    assert!(h.click("agent:mode"));
    h.step();
    assert!(h.gui.shell.agent.mode_menu);
    assert!(
        h.gui.shell.place("agent:mode:plan").is_some(),
        "the three, each said"
    );
    // a misclick doesn't switch it: the mode's the same until a row is picked
    assert_eq!(h.gui.shell.agent.mode, Mode::Ask);
    h.key(egui::Key::Escape);
    h.step();
    assert!(!h.gui.shell.agent.mode_menu, "Esc closes the menu");
    // and only the menu: the question still waits
    assert!(h.gui.shell.agent.waiting().is_some());
    assert!(h.click("agent:mode"));
    h.step();
    assert!(h.click("agent:mode:plan"));
    h.step();
    assert_eq!(h.gui.shell.agent.mode, Mode::Plan);
    assert!(!h.gui.shell.agent.mode_menu);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn enter_answers_only_a_card_in_sight_and_the_chip_brings_it_back() {
    use forge_render::shell::agent::ToolState;
    let Some((mut h, dir)) = app("sight") else { return };
    // a long chat above the question, and the reader scrolled up into it
    let mut long = Vec::new();
    for k in 0..40 {
        long.push(Entry::Tool {
            id: format!("old{k}"),
            name: "Read".into(),
            summary: format!("Read part{k}.fg"),
            input: String::new(),
            output: String::new(),
            state: ToolState::Done,
        });
    }
    h.gui.shell.agent.entries.splice(0..0, long);
    for _ in 0..4 {
        h.step();
    }
    assert!(h.gui.shell.agent.card_visible);
    // the pointer over the chat, where it's in view: on the card
    let chat = h.gui.shell.place("agent:perm:req-1").unwrap();
    h.move_to(chat.center().x, chat.center().y);
    for _ in 0..30 {
        h.wheel(400.0);
        h.step();
    }
    for _ in 0..4 {
        h.step();
    }
    assert!(!h.gui.shell.agent.card_visible, "scrolled out of sight");
    assert!(h.gui.shell.place("agent:jump").is_some(), "Permission needed ↓");
    // Enter in the empty box doesn't answer what can't be seen: it brings it back
    assert!(h.click("agent:input"));
    h.key(egui::Key::Enter);
    for _ in 0..6 {
        h.step();
    }
    assert!(h.gui.shell.agent.waiting().is_some(), "not answered unseen");
    assert!(h.gui.shell.agent.card_visible, "back in sight");
    h.key(egui::Key::Enter);
    h.step();
    assert!(h.gui.shell.agent.waiting().is_none(), "now Enter allows it");
    let _ = std::fs::remove_dir_all(dir);
}

/// UX review pictures for the follow-ups: the card's headline and coloured diff, the mode's
/// menu, and the chip that brings a question scrolled out of sight back. Dark at 1400, light
/// at 480. `FORGE_UI_SHOTS=DIR` writes them.
#[test]
fn polish_pictures() {
    use forge_render::shell::agent::ToolState;
    for (w, dark, tag) in [(1400.0, true, "1400-dark"), (480.0, false, "480-light")] {
        let Ok(mut h) = forge_render::gui::Headless::new((w, 860.0), 1.0) else {
            return;
        };
        let dir = scratch(&format!("polish-{tag}"));
        std::fs::write(dir.join("plate.fg"), PART).unwrap();
        h.gui.agent.program = Some(fake_claude(&dir).display().to_string());
        h.gui.agent.cwd = Some(dir.join("cwd"));
        h.gui.agent.settings_file = Some(dir.join("agent.toml"));
        let mut s = h.gui.shell.settings.clone();
        s.dark = dark;
        h.gui.shell.use_settings(s);
        h.step();
        h.gui.open(dir.join("plate.fg").to_str().unwrap());
        h.settle();
        assert!(h.click("side:tab:Agent"));
        assert!(h.click("agent:input"));
        h.text("make the base plate thicker");
        assert!(h.click("agent:send"));
        assert!(wait(&mut h, |g| g.shell.agent.waiting().is_some()));
        shot(&mut h, &format!("polish-{tag}-card-headline"));
        assert!(h.click("agent:mode"));
        shot(&mut h, &format!("polish-{tag}-mode-menu"));
        h.key(egui::Key::Escape);
        let mut long = Vec::new();
        for k in 0..40 {
            long.push(Entry::Tool {
                id: format!("old{k}"),
                name: "Read".into(),
                summary: format!("Read part{k}.fg"),
                input: String::new(),
                output: String::new(),
                state: ToolState::Done,
            });
        }
        h.gui.shell.agent.entries.splice(0..0, long);
        for _ in 0..4 {
            h.step();
        }
        let card = h.gui.shell.place("agent:perm:req-1").unwrap();
        h.move_to(card.center().x, card.center().y);
        for _ in 0..30 {
            h.wheel(400.0);
            h.step();
        }
        shot(&mut h, &format!("polish-{tag}-permission-needed-chip"));
        drop(h);
        let _ = std::fs::remove_dir_all(dir);
    }
}

#[test]
fn an_edit_is_shown_against_the_file_as_it_is() {
    use forge_render::shell::agent::{diff, diff_text, edited};
    let now = "param a = 12\nparam b = 12\nparam c = 3\n";
    let edit = |s: &str| edited("Edit", &Json::parse(s).unwrap(), now);
    // the first of it, or every one with replace_all
    assert_eq!(
        edit(r#"{"old_string":"= 12","new_string":"= 14"}"#).as_deref(),
        Some("param a = 14\nparam b = 12\nparam c = 3\n")
    );
    assert_eq!(
        edit(r#"{"old_string":"= 12","new_string":"= 14","replace_all":true}"#).as_deref(),
        Some("param a = 14\nparam b = 14\nparam c = 3\n")
    );
    // not in the file: nothing to show (Claude Code refuses it too)
    assert_eq!(edit(r#"{"old_string":"= 99","new_string":"= 1"}"#), None);
    let multi = Json::parse(r#"{"edits":[{"old_string":"a = 12","new_string":"a = 1"},{"old_string":"c = 3","new_string":"c = 4"}]}"#).unwrap();
    let after = edited("MultiEdit", &multi, now).unwrap();
    assert_eq!(after, "param a = 1\nparam b = 12\nparam c = 4\n");
    let write = Json::parse(r#"{"content":"param z = 1\n"}"#).unwrap();
    assert_eq!(edited("Write", &write, now).as_deref(), Some("param z = 1\n"));
    assert_eq!(edited("Bash", &write, now), None);
    let shown = diff_text(&diff(now, &after, 0));
    assert!(shown.contains("- param a = 12\n+ param a = 1"), "{shown}");
    assert!(shown.contains("- param c = 3\n+ param c = 4"), "{shown}");
}

#[test]
fn a_bypass_from_the_persons_settings_can_be_left_and_gone_back_to() {
    let Some((mut h, dir)) = app("own") else { return };
    assert!(h.click("agent:perm:req-1:allow"));
    assert!(wait(&mut h, |g| !g.shell.agent.running));
    // their settings bypass permissions (as Claude Code's status says it)
    h.gui.shell.agent.take(Event::Mode("bypassPermissions".into()));
    h.step();
    if let Ok(shots) = std::env::var("FORGE_UI_SHOTS") {
        for _ in 0..6 {
            h.step();
        }
        std::fs::create_dir_all(&shots).unwrap();
        std::fs::write(
            Path::new(&shots).join("approvals-1400-dark-bypassed.png"),
            h.png().unwrap(),
        )
        .unwrap();
    }
    // said even in the narrow panel ("Bypassed"), and a click opens the menu, never switches
    assert!(h.click("agent:mode"), "the bypass in sight");
    h.step();
    assert!(h.click("agent:mode:default"));
    h.step();
    assert_eq!(h.gui.shell.agent.mode, Mode::Ask);
    assert_eq!(h.gui.shell.agent.mode_other, None);
    // and back: the menu's last row, their settings' own
    assert!(h.click("agent:mode"));
    h.step();
    if let Ok(shots) = std::env::var("FORGE_UI_SHOTS") {
        for _ in 0..12 {
            h.step();
        }
        std::fs::create_dir_all(&shots).unwrap();
        std::fs::write(
            Path::new(&shots).join("approvals-1400-dark-own-mode.png"),
            h.png().unwrap(),
        )
        .unwrap();
    }
    assert!(h.click("agent:mode:own"));
    h.step();
    assert_eq!(h.gui.shell.agent.mode_other.as_deref(), Some("bypassPermissions"));
    assert!(wait(&mut h, |_| heard(&dir).iter().any(|j| j
        .get("request")
        .and_then(|r| r.get("mode"))
        .and_then(Json::str)
        == Some("bypassPermissions"))));
    let _ = std::fs::remove_dir_all(dir);
}

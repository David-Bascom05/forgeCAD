//! The Agent tab: Claude Code's stream-json read from captured real output (tests/fixtures/agent,
//! from `claude -p --output-format stream-json --input-format stream-json --verbose
//! --include-partial-messages`, 2.1.318, paths and the session's settings trimmed), the
//! OpenAI-compatible loop against a mock server, and an agent's writes to the open file as one
//! undo step a turn, in the app.

use forge_render::agent::claude::Parser;
use forge_render::agent::openai::{Config, OpenAi};
use forge_render::agent::{Backend, Event, Out, Turns};
use forge_render::shell::agent::{Entry, Line, SideTab};
use std::io::{BufRead, BufReader, Read, Write};
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

fn parse(name: &str) -> (Vec<Event>, Parser) {
    let mut p = Parser::default();
    let evs = fixture(name).lines().flat_map(|l| p.line(l)).collect();
    (evs, p)
}

fn text_of(evs: &[Event]) -> String {
    evs.iter()
        .filter_map(|e| match e {
            Event::Text(t) => Some(t.as_str()),
            _ => None,
        })
        .collect()
}

/// A fresh directory for a test's files.
fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("forgecad-agent-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn an_edit_turn_reads_as_tool_rows_then_the_streamed_reply_once() {
    let (evs, p) = parse("edit_turn.jsonl");
    assert_eq!(p.session.as_deref(), Some("54df2e91-795a-459c-8150-d35b6132aa69"));
    assert_eq!(p.model.as_deref(), Some("claude-opus-5-5"));
    let tools: Vec<&Event> = evs.iter().filter(|e| !matches!(e, Event::Text(_))).collect();
    assert!(matches!(tools[0], Event::ToolStart { name, .. } if name == "Read"));
    assert!(matches!(tools[1], Event::ToolInput { summary, .. } if summary == "Read part_tools.fg"));
    assert!(
        matches!(tools[2], Event::ToolDone { output, error: false, .. } if output.contains("param extrude1.d = 12"))
    );
    assert!(matches!(tools[3], Event::ToolStart { name, .. } if name == "Edit"));
    match tools[4] {
        Event::ToolInput { summary, input, .. } => {
            assert_eq!(summary, "Edited part_tools.fg");
            assert_eq!(input, "- param extrude1.d = 12\n+ param extrude1.d = 14\n");
        }
        e => panic!("{e:?}"),
    }
    assert!(matches!(tools[5], Event::ToolDone { error: false, .. }));
    assert_eq!(
        tools[6],
        &Event::Done {
            error: None,
            stopped: false
        }
    );
    assert_eq!(tools.len(), 7);
    // token by token, and the whole message after isn't said again
    assert!(evs.iter().filter(|e| matches!(e, Event::Text(_))).count() > 3);
    assert_eq!(
        text_of(&evs),
        "I changed `extrude1.d` in `part_tools.fg:9` from 12 to 14."
    );
}

#[test]
fn a_stopped_turn_says_so_and_the_next_one_answers() {
    let (evs, _) = parse("interrupt.jsonl");
    let first = evs.iter().position(|e| matches!(e, Event::Done { .. })).unwrap();
    assert_eq!(
        evs[first],
        Event::Done {
            error: None,
            stopped: true
        }
    );
    assert!(text_of(&evs[..first]).starts_with("one\ntwo"));
    assert_eq!(text_of(&evs[first..]), "hi");
    assert_eq!(
        evs.last(),
        Some(&Event::Done {
            error: None,
            stopped: false
        })
    );
}

#[test]
fn not_signed_in_and_a_refused_model_say_why_once() {
    let (evs, _) = parse("not_logged_in.jsonl");
    assert_eq!(
        evs,
        vec![
            Event::Problem {
                text: "Claude Code isn't signed in.".into(),
                fix: Some("claude auth login".into())
            },
            Event::Done {
                error: None,
                stopped: false
            }
        ]
    );
    let (evs, _) = parse("bad_model.jsonl");
    assert!(matches!(&evs[0], Event::Note { text, error: true } if text.contains("nonsense-model-x")));
    assert_eq!(evs.len(), 2);
}

/// Each request read and answered with the next body, as an event stream; what was asked, kept.
fn mock(replies: Vec<String>) -> (String, std::sync::mpsc::Receiver<String>) {
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/v1", l.local_addr().unwrap());
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        for body in replies {
            let (mut s, _) = l.accept().unwrap();
            let mut r = BufReader::new(s.try_clone().unwrap());
            let mut head = String::new();
            let mut len = 0;
            loop {
                let mut line = String::new();
                r.read_line(&mut line).unwrap();
                if let Some(v) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                    len = v.trim().parse().unwrap();
                }
                head.push_str(&line);
                if line == "\r\n" {
                    break;
                }
            }
            let mut b = vec![0; len];
            r.read_exact(&mut b).unwrap();
            tx.send(head + &String::from_utf8(b).unwrap()).unwrap();
            write!(
                s,
                "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n{body}"
            )
            .unwrap();
        }
    });
    (url, rx)
}

fn sse(chunks: &[&str]) -> String {
    let mut s: String = chunks.iter().map(|c| format!("data: {c}\n\n")).collect();
    s.push_str("data: [DONE]\n\n");
    s
}

#[test]
fn an_openai_compatible_endpoint_edits_the_file_through_its_tools() {
    let dir = scratch("openai");
    let file = dir.join("plate.fg");
    std::fs::write(&file, PART).unwrap();
    let first = sse(&[
        r#"{"choices":[{"index":0,"delta":{"role":"assistant","content":"Making it "}}]}"#,
        r#"{"choices":[{"index":0,"delta":{"content":"thicker."}}]}"#,
        r#"{"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"id":"call_1","type":"function","function":{"name":"replace_text","arguments":"{\"old\":\"param extrude1.d"}}]}}]}"#,
        r#"{"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":" = 12\",\"new\":\"param extrude1.d = 22\"}"}}]}}]}"#,
        r#"{"choices":[{"index":0,"delta":{},"finish_reason":"tool_calls"}]}"#,
    ]);
    let second = sse(&[
        r#"{"choices":[{"index":0,"delta":{"content":"Done: 22 mm."}}]}"#,
        r#"{"choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}"#,
    ]);
    let (url, asked) = mock(vec![first, second]);
    let (tx, rx) = channel();
    let out = Out {
        tx,
        wake: Arc::new(|| {}),
    };
    let mut b = OpenAi::new(
        Config {
            base_url: url,
            model: "qwen-test".into(),
            file: file.clone(),
            system: "You are a test.".into(),
            key: Some("sk-test".into()),
        },
        out,
    );
    b.send("make the base plate 10 mm thicker".into());
    let mut evs = Vec::new();
    while let Ok(e) = rx.recv_timeout(std::time::Duration::from_secs(60)) {
        let done = matches!(e, Event::Done { .. });
        evs.push(e);
        if done {
            break;
        }
    }
    assert_eq!(text_of(&evs), "Making it thicker.Done: 22 mm.");
    assert!(evs.contains(&Event::ToolStart {
        id: "call_1".into(),
        name: "replace_text".into()
    }));
    assert!(
        evs.iter()
            .any(|e| matches!(e, Event::FileChanged(t) if t.contains("extrude1.d = 22")))
    );
    assert!(evs.iter().any(
        |e| matches!(e, Event::ToolDone { output, error: false, .. } if output.starts_with("Written."))
    ));
    assert_eq!(
        evs.last(),
        Some(&Event::Done {
            error: None,
            stopped: false
        })
    );
    assert!(
        std::fs::read_to_string(&file)
            .unwrap()
            .contains("param extrude1.d = 22")
    );
    let req1 = asked.recv().unwrap();
    assert!(req1.starts_with("POST /v1/chat/completions"), "{req1}");
    assert!(req1.contains("Bearer sk-test"));
    assert!(
        req1.contains("\"model\":\"qwen-test\"")
            && req1.contains("\"replace_text\"")
            && req1.contains("\"stream\":true")
    );
    // the second request carries the call and its answer
    let req2 = asked.recv().unwrap();
    assert!(req2.contains("\"tool_call_id\":\"call_1\"") && req2.contains("\"role\":\"tool\""));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn the_tools_touch_only_the_open_file_and_refuse_what_is_not_there() {
    let dir = scratch("tools");
    let file = dir.join("p.fg");
    std::fs::write(&file, PART).unwrap();
    let j = |s: &str| forge_agent::json::Json::parse(s).unwrap();
    let (got, err) =
        forge_render::agent::tools::call("replace_text", &j(r#"{"old":"nope","new":"x"}"#), &file);
    assert!(err && got.contains("isn't in the file"), "{got}");
    let (got, err) =
        forge_render::agent::tools::call("replace_text", &j(r#"{"old":"param sk1","new":"x"}"#), &file);
    assert!(err && got.contains("4 times"), "{got}");
    let (got, err) = forge_render::agent::tools::call("delete_file", &j("{}"), &file);
    assert!(err && got.contains("no tool"), "{got}");
    let (got, err) = forge_render::agent::tools::call("run_check", &j("{}"), &file);
    assert!(!err, "{got}");
    assert_eq!(std::fs::read_to_string(&file).unwrap(), PART);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_turns_writes_are_one_undo_step_and_its_undo_takes_them_back() {
    let mut s = forge_render::fg::Session::new(None, PART.into(), None, Arc::new(|| {}));
    let mut turns = Turns::default();
    turns.start("Agent: make it thicker".into(), s.src.clone());
    let mid = PART.replace("= 12", "= 18");
    let end = PART.replace("= 12", "= 22");
    assert!(turns.changed(&mut s, mid));
    assert!(turns.changed(&mut s, end.clone()));
    assert!(!s.dirty(), "the file holds what the agent wrote");
    let (k, lines) = turns.finish(&s).unwrap();
    assert!(lines.contains(&Line::Del("param extrude1.d = 12".into())));
    assert!(lines.contains(&Line::Add("param extrude1.d = 22".into())));
    assert_eq!(s.undo_tag().map(|t| t.0), Some("Agent: make it thicker"));
    // one Undo goes back past both writes
    assert!(s.undo());
    assert_eq!(s.src, PART);
    assert!(s.redo());
    assert_eq!(s.src, end);
    // the chat's Undo: the session's own while it's the last step
    assert!(turns.flip(&mut s, k, true));
    assert_eq!(s.src, PART);
    assert!(turns.flip(&mut s, k, false));
    assert_eq!(s.src, end);
    // the next turn is a step of its own
    turns.start("Agent: and wider".into(), s.src.clone());
    let wider = end.replace("w3 = 80", "w3 = 90");
    assert!(turns.changed(&mut s, wider));
    turns.finish(&s).unwrap();
    assert!(s.undo());
    assert_eq!(s.src, end);
}

/// A stand-in `claude`: the captured edit turn, its Edit made on the open file in between.
fn fake_claude(dir: &Path) -> PathBuf {
    let lines: Vec<String> = fixture("edit_turn.jsonl").lines().map(str::to_string).collect();
    let cut = lines
        .iter()
        .position(|l| l.contains("has been updated successfully"))
        .unwrap();
    std::fs::write(dir.join("head.jsonl"), lines[..cut].join("\n") + "\n").unwrap();
    std::fs::write(dir.join("tail.jsonl"), lines[cut..].join("\n") + "\n").unwrap();
    let script = dir.join("claude");
    std::fs::write(
        &script,
        format!(
            "#!/bin/sh\nread line\ncat '{0}/head.jsonl'\nsed -i 's/^param extrude1.d = 12$/param extrude1.d = 14/' plate.fg\ncat '{0}/tail.jsonl'\nwhile read line; do :; done\n",
            dir.display()
        ),
    )
    .unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    script
}

#[test]
fn in_the_app_a_turn_reloads_the_part_as_one_undo_step_and_the_tabs_keep_their_state() {
    let Ok(mut h) = forge_render::gui::Headless::new((1400.0, 900.0), 1.0) else {
        return;
    };
    let dir = scratch("app");
    let file = dir.join("plate.fg");
    std::fs::write(&file, PART).unwrap();
    h.gui.agent.program = Some(fake_claude(&dir).display().to_string());
    h.step();
    h.gui.open(file.to_str().unwrap());
    h.settle();
    h.step();
    assert!(
        h.gui.shell.place("fg:row:extrude1").is_some(),
        "the Model tab's browser"
    );
    assert!(h.click("side:tab:Agent"));
    h.step();
    assert_eq!(h.gui.shell.agent.tab, SideTab::Agent);
    assert!(h.gui.shell.place("fg:row:extrude1").is_none());
    assert!(h.gui.shell.place("agent:input").is_some());
    assert!(h.click("agent:input"));
    h.step();
    h.text("make the base plate thicker");
    h.step();
    // the other tab and back: what was typed is still there
    assert!(h.click("side:tab:Model"));
    h.step();
    assert!(h.gui.shell.place("fg:row:extrude1").is_some());
    assert!(h.click("side:tab:Agent"));
    h.step();
    assert_eq!(h.gui.shell.agent.input, "make the base plate thicker");
    assert!(h.click("agent:send"));
    assert!(h.until(|g| {
        !g.shell.agent.running
            && g.shell
                .agent
                .entries
                .iter()
                .any(|e| matches!(e, Entry::Diff { .. }))
    }));
    let s = h.gui.fg.as_ref().unwrap();
    assert!(s.src.contains("param extrude1.d = 14"));
    assert_eq!(
        s.undo_tag().map(|t| t.0.to_string()),
        Some("Agent: make the base plate thicker".into())
    );
    let kinds: Vec<&str> = h
        .gui
        .shell
        .agent
        .entries
        .iter()
        .map(|e| match e {
            Entry::User { .. } => "user",
            Entry::Agent { .. } => "agent",
            Entry::Tool { .. } => "tool",
            Entry::Diff { .. } => "diff",
            Entry::Note { .. } => "note",
        })
        .collect();
    assert_eq!(kinds, ["user", "tool", "tool", "agent", "diff"]);
    // the diff's Undo is the app's one step back
    assert!(h.click("agent:undo:0"));
    h.step();
    assert_eq!(h.gui.fg.as_ref().unwrap().src, PART);
    assert!(
        h.gui
            .shell
            .agent
            .entries
            .iter()
            .any(|e| matches!(e, Entry::Diff { undone: true, .. }))
    );
    drop(h);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
#[ignore = "writes to the session's keyring: run by hand"]
fn a_key_goes_into_the_keyring_and_out_again() {
    use forge_render::agent::keys;
    let url = "http://forgecad-test.invalid/v1";
    keys::set(url, "sk-round-trip").unwrap();
    assert_eq!(keys::get(url).unwrap().as_deref(), Some("sk-round-trip"));
    keys::set(url, "").unwrap();
    assert_eq!(keys::get(url).unwrap(), None);
}

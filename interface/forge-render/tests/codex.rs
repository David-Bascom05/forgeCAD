//! Codex's event stream, process lifecycle, saved settings, and the app's selector.
use forge_render::agent::{
    Backend, Event, Out,
    claude::Config,
    codex::{Codex, Parser},
};
use forge_render::shell::agent_settings::{AgentSettings, Kind};
use std::path::PathBuf;
use std::sync::{
    Arc,
    mpsc::{Receiver, channel},
};
use std::time::Duration;

fn scratch() -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let dir = std::env::temp_dir().join(format!(
        "forgecad-codex-test-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn backend(dir: &std::path::Path, script: &str) -> (Codex, Receiver<Event>) {
    use std::os::unix::fs::PermissionsExt;
    let program = dir.join("fake-codex");
    std::fs::write(&program, script).unwrap();
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
    std::fs::write(dir.join("part.fg"), "before").unwrap();
    let (tx, rx) = channel();
    (
        Codex::new(
            Config {
                program: program.display().to_string(),
                model: String::new(),
                file: dir.join("part.fg"),
                system: "Use the app tools".into(),
                cwd: dir.join("clean cwd"),
                mcp: Some(forge_render::agent::mcp::config(
                    std::path::Path::new("/a path/forge-app"),
                    std::path::Path::new("/tmp/app.sock"),
                )),
            },
            Out {
                tx,
                wake: Arc::new(|| {}),
            },
            None,
        ),
        rx,
    )
}

fn until_done(rx: &Receiver<Event>) -> Vec<Event> {
    let mut events = vec![];
    loop {
        let e = rx
            .recv_timeout(Duration::from_secs(15))
            .expect("backend finished");
        let done = matches!(e, Event::Done { .. } | Event::Problem { .. });
        events.push(e);
        if done {
            return events;
        }
    }
}

#[test]
fn codex_json_maps_tools_text_failures_and_thread_id() {
    let mut p = Parser::default();
    assert!(p.line("not json").is_empty());
    p.line(r#"{"type":"thread.started","thread_id":"thread-1"}"#);
    assert_eq!(p.session.as_deref(), Some("thread-1"));
    let start = p.line(r#"{"type":"item.started","item":{"id":"tool-1","type":"mcp_tool_call","server":"forgecad","tool":"fg_run","arguments":{"file":"part.fg"}}}"#);
    assert!(matches!(&start[0], Event::ToolStart { name, .. } if name == "mcp__forgecad__fg_run"));
    assert!(matches!(&start[1], Event::ToolInput { input, .. } if input.contains("part.fg")));
    let end = p.line(r#"{"type":"item.completed","item":{"id":"tool-1","type":"mcp_tool_call","server":"forgecad","tool":"fg_run","status":"completed","result":{"content":[{"text":"valid"}],"isError":false}}}"#);
    assert_eq!(end.len(), 1);
    assert!(matches!(&end[0], Event::ToolDone { error: false, output, .. } if output.contains("valid")));
    let change = p.line(r#"{"type":"item.completed","item":{"id":"file-1","type":"file_change","changes":[{"path":"part.fg","kind":"update"}],"status":"completed"}}"#);
    assert!(matches!(&change[0], Event::ToolStart { .. }));
    assert!(matches!(&change[2], Event::ToolDone { output, .. } if output.contains("part.fg")));
    assert_eq!(p.line(r#"{"type":"item.completed","item":{"id":"msg","type":"agent_message","text":"Updated the part."}}"#), [Event::Text("Updated the part.\n\n".into())]);
    p.line(r#"{"type":"turn.failed","error":{"message":"Please sign in"}}"#);
    assert_eq!(p.error.as_deref(), Some("Please sign in"));
}

#[test]
fn codex_settings_round_trip_in_both_formats() {
    let s = AgentSettings {
        kind: Kind::Codex,
        codex_model: "a-model".into(),
        ..Default::default()
    };
    assert_eq!(AgentSettings::from_toml(&s.to_toml()), s);
    let mut settings = forge_render::shell::settings::Settings::default();
    s.write_extra(&mut settings.extra);
    let back = forge_render::shell::settings::Settings::from_toml(&settings.to_toml());
    assert_eq!(AgentSettings::from_extra(&back.extra), s);
    assert_eq!(s.label(), "Codex · a-model");
    assert_eq!(forge_render::shell::agent_settings::missing(&s), None);
}

#[test]
fn codex_process_resumes_reports_file_changes_and_cleans_images() {
    let dir = scratch();
    let (mut b, rx) = backend(
        &dir,
        r#"#!/bin/sh
cat > prompt.txt
printf '%s\n' "$@" > args.txt
printf '{"type":"thread.started","thread_id":"saved-thread"}\n'
printf 'after' > ../part.fg
printf '{"type":"item.completed","item":{"id":"m","type":"agent_message","text":"Done"}}\n'
printf '{"type":"turn.completed","usage":{}}\n'
"#,
    );
    b.send("Make a change".into(), Some(vec![1, 2, 3]));
    let events = until_done(&rx);
    assert!(matches!(
        events.last(),
        Some(Event::Done {
            error: None,
            stopped: false
        })
    ));
    assert!(events.iter().any(|e| matches!(e, Event::FileChanged { before: Some(before), text, .. } if before == "before" && text == "after")));
    assert_eq!(
        std::fs::read_to_string(dir.join("clean cwd/prompt.txt")).unwrap(),
        "Make a change"
    );
    assert_eq!(*b.session.lock().unwrap(), Some("saved-thread".into()));
    let args = b.args(None);
    assert!(args.windows(2).any(|a| a == ["resume", "saved-thread"]));
    assert!(
        args.iter()
            .any(|a| a.contains("mcp_servers.forgecad={command=\"/a path/forge-app\""))
    );
    assert!(
        args.iter()
            .any(|a| a.contains("default_tools_approval_mode=\"approve\""))
    );
    assert!(args.iter().any(|a| a == "--ignore-user-config"));
    assert!(
        std::fs::read_dir(dir.join("clean cwd")).unwrap().all(|p| p
            .unwrap()
            .path()
            .extension()
            .is_none_or(|x| x != "png"))
    );
    b.send("Follow up".into(), None);
    assert!(matches!(
        until_done(&rx).last(),
        Some(Event::Done {
            error: None,
            stopped: false
        })
    ));
    drop(b);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn codex_stop_and_exit_failures_finish_the_turn() {
    let dir = scratch();
    let (mut b, rx) = backend(&dir, "#!/bin/sh\ncat >/dev/null\nexec sleep 30\n");
    b.send("Wait".into(), None);
    b.stop();
    assert!(matches!(
        until_done(&rx).last(),
        Some(Event::Done {
            error: None,
            stopped: true
        })
    ));
    drop(b);
    let (mut b, rx) = backend(
        &dir,
        "#!/bin/sh\ncat >/dev/null\necho 'Not signed in' >&2\nexit 1\n",
    );
    b.send("Hi".into(), None);
    assert!(
        matches!(until_done(&rx).last(), Some(Event::Done { error: Some(e), stopped: false }) if e.contains("Not signed in"))
    );
    drop(b);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn codex_is_selectable_in_the_agent_switcher() {
    let Ok(mut h) = forge_render::gui::Headless::new((1400.0, 900.0), 1.0) else {
        return;
    };
    h.step();
    assert!(h.click("side:tab:Agent"));
    h.step();
    assert!(h.click("agent:switch"));
    h.step();
    assert!(h.click("agent:switch:codex"));
    h.step();
    h.step();
    assert_eq!(h.gui.shell.agent.settings.kind, Kind::Codex);
    assert_eq!(h.gui.shell.agent.settings.label(), "Codex");
    assert_eq!(h.gui.shell.settings.extra.get("agent", "backend"), Some("codex"));
}

/// Explicit opt-in: uses the installed Codex CLI and the person's login.
#[test]
#[ignore = "live Codex login and model access required"]
fn codex_live_uses_the_apps_mcp_tools_and_resumes_with_an_image() {
    let dir = scratch();
    let (tools, calls) = forge_render::agent::apptools::AppTools::new(Arc::new(|| {}));
    tools.set_dir(&dir);
    let socket = dir.join("app.sock");
    forge_render::agent::mcp::serve(&socket, tools).unwrap();
    std::thread::spawn(move || {
        for call in calls {
            let _ = call.reply.send(forge_render::agent::apptools::Reply::ok(
                "ForgeCAD test tree: part named plate",
            ));
        }
    });
    let (tx, rx) = channel();
    let mut b = Codex::new(Config {
        program: "codex".into(), model: String::new(), file: dir.join("part.fg"),
        system: "You are ForgeCAD's test agent. Use the forgecad MCP tools when asked. Do not run shell commands or modify files. Reply in one sentence.".into(),
        cwd: dir.join("cwd"),
        mcp: Some(forge_render::agent::mcp::config(std::path::Path::new(env!("CARGO_BIN_EXE_forge-app")), &socket)),
    }, Out { tx, wake: Arc::new(|| {}) }, None);
    let collect = || {
        let mut events = Vec::new();
        loop {
            let event = rx
                .recv_timeout(Duration::from_secs(120))
                .expect("live turn completes");
            let done = matches!(event, Event::Done { .. } | Event::Problem { .. });
            events.push(event);
            if done {
                return events;
            }
        }
    };
    b.send(
        "Call forgecad's tree tool and tell me the part's name.".into(),
        None,
    );
    let events = collect();
    assert!(
        matches!(
            events.last(),
            Some(Event::Done {
                error: None,
                stopped: false
            })
        ),
        "{events:?}"
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::ToolDone { error: false, output, .. } if output.contains("plate"))),
        "{events:?}"
    );
    let session = b.session.lock().unwrap().clone().expect("thread id");
    let png = forge_render::agent::apptools::png(16, 16, &[255, 0, 0].repeat(256)).unwrap();
    b.send("What colour is the attached image, and what part name did the tree return last turn? Do not call tools.".into(), Some(png));
    let events = collect();
    assert!(
        matches!(
            events.last(),
            Some(Event::Done {
                error: None,
                stopped: false
            })
        ),
        "{events:?}"
    );
    let reply: String = events
        .iter()
        .filter_map(|e| match e {
            Event::Text(t) => Some(t.as_str()),
            _ => None,
        })
        .collect();
    assert!(
        reply.to_lowercase().contains("red") && reply.to_lowercase().contains("plate"),
        "{reply}"
    );
    assert_eq!(b.session.lock().unwrap().as_ref(), Some(&session));
    drop(b);
    std::fs::remove_dir_all(dir).unwrap();
}

//! Saved project chats, native Codex import, and resume through the rendered Agent tab.
use forge_agent::json::Json;
use forge_render::agent::conversations::{self, Conversation};
use forge_render::shell::{
    agent::{AgentAct, Entry, ToolState},
    agent_settings::{AgentSettings, Kind},
};
use std::path::{Path, PathBuf};

fn scratch() -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let dir = std::env::temp_dir().join(format!(
        "forgecad-conversations-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir.canonicalize().unwrap()
}
fn settings() -> AgentSettings {
    AgentSettings {
        kind: Kind::Codex,
        codex_model: "model-id".into(),
        trusted: vec!["/not-an-inherited-trust".into()],
        ..Default::default()
    }
}

#[test]
fn chats_round_trip_locally_and_stay_scoped_to_their_project() {
    use std::os::unix::fs::PermissionsExt;
    let project = scratch();
    let other = scratch();
    let storage = scratch();
    let mut chat = Conversation::new(&project, &settings(), "Telescope mount\nMore text").unwrap();
    chat.session = Some("saved-native-session".into());
    chat.entries = vec![
        Entry::User {
            text: "Use a fork mount".into(),
            refs: vec!["base".into()],
            markup: None,
        },
        Entry::Agent {
            text: "The fork mount is ready.".into(),
        },
        Entry::Tool {
            id: "t".into(),
            name: "project_write".into(),
            summary: "Wrote part".into(),
            input: "{}".into(),
            output: "ok".into(),
            state: ToolState::Running,
        },
        Entry::Diff {
            turn: 0,
            index: 0,
            created: true,
            file: "fork.fg".into(),
            lines: vec![],
            added: 20,
            removed: 0,
            undone: false,
        },
    ];
    chat.history = vec![Json::obj(vec![
        ("role", "user".into()),
        ("content", "Prior endpoint message".into()),
    ])];
    chat.interrupted = true;
    chat.save(&storage).unwrap();
    let saved = conversations::load(&storage, &project, &chat.id).unwrap();
    assert_eq!(saved.title, "Telescope mount");
    assert_eq!(saved.session, chat.session);
    assert_eq!(saved.history, chat.history);
    assert!(saved.settings.trusted.is_empty());
    assert_eq!(saved.settings.codex_model, "model-id");
    assert!(matches!(
        saved.entries[2],
        Entry::Tool {
            state: ToolState::Failed,
            ..
        }
    ));
    assert!(matches!(&saved.entries[3],Entry::Note{text,..} if text.contains("fork.fg")));
    assert!(
        !saved.entries.iter().any(|e| matches!(e, Entry::Diff { .. })),
        "restoring a transcript must not offer stale undo actions"
    );
    assert_eq!(conversations::list(&storage, &project).unwrap().len(), 1);
    assert!(conversations::list(&storage, &other).unwrap().is_empty());
    assert!(conversations::load(&storage, &other, &chat.id).is_err());
    for id in ["../escape", "/tmp/other", "", "a/b"] {
        assert!(conversations::load(&storage, &project, id).is_err());
    }
    let folder = std::fs::read_dir(&storage)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let path = folder.join(&chat.id).join("chat.json");
    assert_eq!(
        std::fs::metadata(path).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert!(
        std::fs::read_dir(&project).unwrap().next().is_none(),
        "no chat files enter the project"
    );
    std::fs::remove_dir_all(project).unwrap();
    std::fs::remove_dir_all(other).unwrap();
    std::fs::remove_dir_all(storage).unwrap();
}

#[test]
fn concurrent_windows_cannot_silently_overwrite_a_saved_conversation() {
    let project = scratch();
    let storage = scratch();
    let mut chat = Conversation::new(&project, &settings(), "First title").unwrap();
    chat.save(&storage).unwrap();
    let mut stale = conversations::load(&storage, &project, &chat.id).unwrap();
    chat.title = "Updated in another window".into();
    chat.save(&storage).unwrap();
    stale.title = "Stale title".into();
    assert!(stale.save(&storage).is_err());
    assert_eq!(
        conversations::load(&storage, &project, &chat.id).unwrap().title,
        "Updated in another window"
    );
    std::fs::remove_dir_all(project).unwrap();
    std::fs::remove_dir_all(storage).unwrap();
}

#[test]
fn endpoint_history_preserves_completed_tool_calls_and_drops_interrupted_groups() {
    let j = |s| Json::parse(s).unwrap();
    let prior = j(r#"{"role":"user","content":"First question"}"#);
    let calls = j(
        r#"{"role":"assistant","tool_calls":[{"id":"tool1","function":{"name":"project_info","arguments":"{}"}}]}"#,
    );
    let result = j(r#"{"role":"tool","tool_call_id":"tool1","content":"ok"}"#);
    assert_eq!(
        conversations::complete_history(vec![prior.clone(), calls.clone()]),
        [prior.clone()]
    );
    let complete = vec![prior, calls, result];
    assert_eq!(conversations::complete_history(complete.clone()), complete);
}

#[test]
fn old_codex_logs_are_imported_read_only_and_only_for_the_matching_project() {
    let project = scratch();
    let other = scratch();
    let storage = scratch();
    let native = scratch();
    let cwd = scratch();
    let write = |id: &str, root: &Path| {
        let rows=vec![
            Json::obj(vec![("type","session_meta".into()),("payload",Json::obj(vec![("id",id.into()),("cwd",cwd.display().to_string().into())]))]),
            Json::obj(vec![("type","response_item".into()),("payload",Json::obj(vec![("role","developer".into()),("content",Json::Arr(vec![Json::obj(vec![("text",format!("You are ForgeCAD's in-app agent. The project root is {}. All app file paths are relative to this root.",root.display()).into())])]))]))]),
            Json::obj(vec![("type","response_item".into()),("payload",Json::obj(vec![("role","user".into()),("content",Json::Arr(vec![Json::obj(vec![("text","Build a telescope".into())])]))]))]),
            Json::obj(vec![("type","response_item".into()),("payload",Json::obj(vec![("role","assistant".into()),("content",Json::Arr(vec![Json::obj(vec![("text","We can start with the mount.".into())])]))]))]),
            Json::obj(vec![("type","turn_context".into()),("payload",Json::obj(vec![("model","native-model".into())]))]),
        ];
        let text = rows.iter().map(|j| format!("{j}\n")).collect::<String>();
        std::fs::write(native.join(format!("{id}.jsonl")), &text).unwrap();
        text
    };
    let original = write("abcd-1234", &project);
    write("abcd-5678", &other);
    assert_eq!(
        conversations::import_codex_from(&storage, &project, &native, &cwd).unwrap(),
        1
    );
    assert_eq!(
        conversations::import_codex_from(&storage, &project, &native, &cwd).unwrap(),
        0
    );
    let list = conversations::list(&storage, &project).unwrap();
    assert_eq!(list.len(), 1);
    let chat = conversations::load(&storage, &project, &list[0].id).unwrap();
    assert_eq!(chat.session.as_deref(), Some("abcd-1234"));
    assert_eq!(chat.settings.codex_model, "native-model");
    assert_eq!(chat.entries.len(), 2);
    assert_eq!(
        std::fs::read_to_string(native.join("abcd-1234.jsonl")).unwrap(),
        original
    );
    for p in [project, other, storage, native, cwd] {
        std::fs::remove_dir_all(p).unwrap();
    }
}

fn view(root: &Path, storage: &Path, program: &Path) -> forge_render::gui::Headless {
    let mut view = forge_render::gui::Headless::new((1400.0, 900.0), 1.0).unwrap();
    view.gui.agent.project_dir = Some(root.to_path_buf());
    view.gui.agent.conversations_dir = Some(storage.to_path_buf());
    view.gui.agent.settings_file = Some(root.join("unused-settings.toml"));
    view.gui.agent.cwd = Some(root.join(".agent-run"));
    view.gui.agent.program = Some(program.display().to_string());
    view.step();
    AgentSettings {
        kind: Kind::Codex,
        trusted: vec![root.display().to_string()],
        ..Default::default()
    }
    .write_extra(&mut view.gui.shell.settings.extra);
    view.step();
    assert!(view.click("side:tab:Agent"));
    view.step();
    view
}
fn send(view: &mut forge_render::gui::Headless, text: &str) {
    assert!(view.click("agent:input"));
    view.step();
    view.text(text);
    view.step();
    assert!(view.click("agent:send"));
    view.step();
}
fn wait(view: &mut forge_render::gui::Headless) {
    let started = std::time::Instant::now();
    while view.gui.shell.agent.running {
        assert!(
            started.elapsed().as_secs() < 30,
            "{:?}",
            view.gui.shell.agent.entries
        );
        view.step();
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    assert!(
        view.gui.shell.agent.problem.is_none(),
        "{:?}",
        view.gui.shell.agent.problem
    );
}

#[test]
fn resume_picker_restores_the_transcript_and_native_session_after_reopening_the_app() {
    use std::os::unix::fs::PermissionsExt;
    let root = scratch();
    let storage = scratch();
    let program = root.join("fake-codex");
    std::fs::write(&program,r#"#!/bin/sh
cat > prompt.txt
printf '%s\n' "$@" >> arguments.txt
printf '{"type":"thread.started","thread_id":"saved-session"}\n'
printf '{"type":"item.completed","item":{"id":"m","type":"agent_message","text":"The telescope mount discussion is saved."}}\n'
printf '{"type":"turn.completed","usage":{}}\n'
"#).unwrap();
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
    let mut first = view(&root, &storage, &program);
    send(&mut first, "First mount discussion");
    wait(&mut first);
    // Browsing history after selecting another provider must not relabel the old session.
    let first_id = conversations::list(&storage, &root).unwrap()[0].id.clone();
    let mut changed = first.gui.shell.agent.settings.clone();
    changed.kind = Kind::OpenAi;
    changed.write_extra(&mut first.gui.shell.settings.extra);
    first.step();
    first.gui.shell.agent.out.push(AgentAct::ListConversations);
    first.step();
    assert_eq!(
        conversations::load(&storage, &root, &first_id)
            .unwrap()
            .settings
            .kind,
        Kind::Codex
    );
    first.gui.shell.agent.resume_open = false;
    changed.kind = Kind::Codex;
    changed.write_extra(&mut first.gui.shell.settings.extra);
    first.step();
    send(&mut first, "/new");
    first.step();
    assert!(first.gui.shell.agent.entries.is_empty());
    send(&mut first, "Second optical discussion");
    wait(&mut first);
    drop(first);
    let summaries = conversations::list(&storage, &root).unwrap();
    assert_eq!(summaries.len(), 2);
    let id = summaries
        .iter()
        .find(|s| s.title == "First mount discussion")
        .unwrap()
        .id
        .clone();
    let before = std::fs::read_to_string(root.join(".agent-run/arguments.txt")).unwrap();
    let mut reopened = view(&root, &storage, &program);
    send(&mut reopened, "/resume");
    reopened.step();
    reopened.step();
    assert!(reopened.gui.shell.agent.resume_open);
    assert_eq!(reopened.gui.shell.agent.saved_chats.len(), 2);
    assert!(
        reopened.gui.shell.agent.entries.is_empty(),
        "slash command isn't sent to the model"
    );
    assert!(reopened.click(&format!("agent:resume:{id}")));
    reopened.step();
    reopened.step();
    assert!(!reopened.gui.shell.agent.resume_open);
    assert!(
        reopened
            .gui
            .shell
            .agent
            .entries
            .iter()
            .any(|e| matches!(e,Entry::User{text,..} if text=="First mount discussion"))
    );
    assert!(
        !reopened
            .gui
            .shell
            .agent
            .entries
            .iter()
            .any(|e| matches!(e,Entry::User{text,..} if text=="Second optical discussion"))
    );
    assert_eq!(
        std::fs::read_to_string(root.join(".agent-run/arguments.txt")).unwrap(),
        before,
        "opening a chat does not start an agent turn"
    );
    send(&mut reopened, "Continue the mount discussion");
    wait(&mut reopened);
    let arguments = std::fs::read_to_string(root.join(".agent-run/arguments.txt")).unwrap();
    assert!(arguments[before.len()..].contains("resume\nsaved-session"));
    assert_eq!(
        conversations::list(&storage, &root).unwrap().len(),
        2,
        "resume updates the existing chat"
    );
    drop(reopened);
    let other = scratch();
    let mut isolated = view(&other, &storage, &program);
    assert!(isolated.click("agent:resume"));
    isolated.step();
    isolated.step();
    assert!(isolated.gui.shell.agent.resume_open);
    assert!(isolated.gui.shell.agent.saved_chats.is_empty());
    isolated.gui.shell.agent.resume_open = false;
    isolated.step();
    send(&mut isolated, "/resume invalid-id");
    isolated.step();
    assert!(
        isolated.gui.shell.agent.resume_open && isolated.gui.shell.agent.resume_error.is_some(),
        "invalid resume IDs show an error without invoking the model"
    );
    drop(isolated);
    for p in [root, storage, other] {
        std::fs::remove_dir_all(p).unwrap();
    }
}

#[test]
fn resume_commands_do_not_become_user_messages() {
    let mut tab = forge_render::shell::agent::AgentTab::default();
    tab.input = "/resume".into();
    tab.send();
    assert_eq!(tab.out, [AgentAct::ListConversations]);
    assert!(tab.entries.is_empty());
    assert!(!tab.running);
    tab.out.clear();
    tab.input = "/resume abcd-1234".into();
    tab.send();
    assert_eq!(tab.out, [AgentAct::ResumeConversation("abcd-1234".into())]);
}

#[test]
#[ignore = "reads existing local Codex ForgeCAD logs for an explicitly selected project"]
fn existing_project_codex_logs_can_be_imported() {
    let project =
        PathBuf::from(std::env::var_os("FORGECAD_IMPORT_PROJECT").expect("set FORGECAD_IMPORT_PROJECT"));
    let storage = scratch();
    let count = conversations::import_codex(&storage, &project).unwrap();
    assert!(count > 0, "existing project chats found");
    println!("Imported {count} existing project conversations into temporary test storage");
    for item in conversations::list(&storage, &project).unwrap() {
        let chat = conversations::load(&storage, &project, &item.id).unwrap();
        assert!(chat.session.is_some());
        assert!(!chat.entries.is_empty());
    }
    std::fs::remove_dir_all(storage).unwrap();
}

#[test]
fn restored_endpoint_context_is_sent_to_the_model_on_the_next_turn() {
    use forge_render::agent::{
        Backend, Event, Out,
        openai::{Config, OpenAi},
    };
    use std::io::{BufRead, BufReader, Read, Write};
    let root = scratch();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let (request_tx, request_rx) = std::sync::mpsc::channel();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut length = 0;
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            if line == "\r\n" {
                break;
            }
            if let Some(value) = line.to_lowercase().strip_prefix("content-length:") {
                length = value.trim().parse().unwrap();
            }
        }
        let mut body = vec![0; length];
        reader.read_exact(&mut body).unwrap();
        request_tx
            .send(Json::parse(&String::from_utf8(body).unwrap()).unwrap())
            .unwrap();
        let reply = r#"{"choices":[{"message":{"role":"assistant","content":"Continued from your saved mount discussion."},"finish_reason":"stop"}]}"#;
        write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{reply}",reply.len()).unwrap();
    });
    let (tx, rx) = std::sync::mpsc::channel();
    let mut backend = OpenAi::new(
        Config {
            base_url: format!("http://{address}/v1"),
            model: "test-model".into(),
            file: root.clone(),
            system: "Project agent".into(),
            key: Some(String::new()),
            apps: None,
        },
        Out {
            tx,
            wake: std::sync::Arc::new(|| {}),
        },
    );
    backend.restore_history(vec![
        Json::obj(vec![
            ("role", "user".into()),
            ("content", "Use a fork mount".into()),
        ]),
        Json::obj(vec![
            ("role", "assistant".into()),
            ("content", "A fork mount will work".into()),
        ]),
    ]);
    backend.send("Continue the design".into(), None);
    loop {
        let event = rx.recv_timeout(std::time::Duration::from_secs(15)).unwrap();
        if let Event::Done { error, .. } = event {
            assert!(error.is_none(), "{error:?}");
            break;
        }
    }
    let request = request_rx.recv().unwrap().to_string();
    assert!(
        request.contains("Use a fork mount")
            && request.contains("A fork mount will work")
            && request.contains("Continue the design")
    );
    server.join().unwrap();
    drop(backend);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
#[ignore = "uses installed Codex and the user's model access"]
fn codex_live_resumes_its_saved_project_conversation_after_app_restart() {
    let root = scratch();
    let storage = scratch();
    let mut first = view(&root, &storage, Path::new("codex"));
    first.gui.agent.mcp_exe = Some(PathBuf::from(env!("CARGO_BIN_EXE_forge-app")));
    send(
        &mut first,
        "Remember this project's codename: ORBIT_COBALT_417. Call project_info once, then reply only Stored. Do not write any files.",
    );
    let started = std::time::Instant::now();
    while first.gui.shell.agent.running {
        assert!(
            started.elapsed().as_secs() < 120,
            "{:?}",
            first.gui.shell.agent.entries
        );
        first.step();
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(first.gui.shell.agent.problem.is_none());
    drop(first);
    let saved = conversations::list(&storage, &root).unwrap();
    assert_eq!(saved.len(), 1);
    let id = saved[0].id.clone();
    let original_session = saved[0].session.clone().expect("native session saved");
    let mut reopened = view(&root, &storage, Path::new("codex"));
    reopened.gui.agent.mcp_exe = Some(PathBuf::from(env!("CARGO_BIN_EXE_forge-app")));
    assert!(reopened.click("agent:resume"));
    reopened.step();
    reopened.step();
    assert!(reopened.click(&format!("agent:resume:{id}")));
    reopened.step();
    reopened.step();
    send(
        &mut reopened,
        "Call project_info once, then tell me the project codename I asked you to remember. Reply with only that codename. Do not write any files.",
    );
    let started = std::time::Instant::now();
    while reopened.gui.shell.agent.running {
        assert!(
            started.elapsed().as_secs() < 120,
            "{:?}",
            reopened.gui.shell.agent.entries
        );
        reopened.step();
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(
        reopened.gui.shell.agent.problem.is_none(),
        "{:?}",
        reopened.gui.shell.agent.problem
    );
    let reply = reopened
        .gui
        .shell
        .agent
        .entries
        .iter()
        .rev()
        .find_map(|e| match e {
            Entry::Agent { text } => Some(text.as_str()),
            _ => None,
        })
        .unwrap();
    assert!(reply.contains("ORBIT_COBALT_417"), "{reply}");
    drop(reopened);
    assert_eq!(
        conversations::load(&storage, &root, &id)
            .unwrap()
            .session
            .as_deref(),
        Some(original_session.as_str())
    );
    std::fs::remove_dir_all(root).unwrap();
    std::fs::remove_dir_all(storage).unwrap();
}

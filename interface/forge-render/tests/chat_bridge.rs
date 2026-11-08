use forge_agent::{chat_bridge, json::Json};
use forge_render::{
    gui::Headless,
    shell::agent_settings::{AgentSettings, Kind},
};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
fn request(window: &str, method: &str, key: &str, value: &str) -> Json {
    chat_bridge::call(
        window,
        Json::obj(vec![("method", method.into()), (key, value.into())]),
    )
    .unwrap()
}
fn setup(kind: Kind, name: &str) -> (Headless, PathBuf, String) {
    use std::os::unix::fs::PermissionsExt;
    let root = std::env::temp_dir().join(format!("forgecad-bridge-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let program = root.join("agent");
    std::fs::write(&program, r#"#!/usr/bin/env python3
import sys,json
print_line=lambda x: print(json.dumps(x),flush=True)
if 'exec' in sys.argv:
    sys.stdin.read()
    print_line({'type':'thread.started','thread_id':'bridge-test'})
    print_line({'type':'item.completed','item':{'id':'reply','type':'agent_message','text':'Reply from Codex'}})
    print_line({'type':'turn.completed','usage':{}})
else:
    print_line({'type':'system','subtype':'init','session_id':'bridge-test'})
    for line in sys.stdin:
        print_line({'type':'assistant','message':{'id':'reply','role':'assistant','content':[{'type':'text','text':'Reply from Claude'}]}})
        print_line({'type':'result','subtype':'success','is_error':False,'session_id':'bridge-test','result':'Reply from Claude'})
"#).unwrap();
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
    let mut h = Headless::new((1000.0, 700.0), 1.0).unwrap();
    h.gui.agent.project_dir = Some(root.clone());
    h.gui.agent.program = Some(program.display().to_string());
    h.gui.agent.settings_file = Some(root.join("settings.toml"));
    AgentSettings {
        kind,
        ..Default::default()
    }
    .write_extra(&mut h.gui.shell.settings.extra);
    h.gui.start_agent_bridge();
    h.step();
    let window = h.gui.agent.bridge.as_ref().unwrap().id.clone();
    (h, root, window)
}
#[test]
fn terminal_requests_queue_and_return_replies_from_both_native_backends() {
    for (kind, name, reply) in [
        (Kind::Codex, "codex", "Reply from Codex"),
        (Kind::ClaudeCode, "claude", "Reply from Claude"),
    ] {
        let (mut h, root, window) = setup(kind, name);
        assert!(chat_bridge::list().unwrap().to_string().contains(&window));
        h.gui.shell.agent.input = "My unsent draft".into();
        h.gui.shell.agent.running = true;
        let first = request(
            &window,
            "send",
            "message",
            "Terminal Claude asks UI GPT to review",
        );
        let second = request(
            &window,
            "send",
            "message",
            "Terminal GPT asks UI Claude to review",
        );
        h.step();
        assert_eq!(
            request(
                &window,
                "status",
                "request",
                first.get("request").unwrap().str().unwrap()
            )
            .get("state")
            .and_then(Json::str),
            Some("queued")
        );
        h.gui.shell.agent.running = false;
        let t = Instant::now();
        loop {
            h.step();
            let status = request(
                &window,
                "status",
                "request",
                second.get("request").unwrap().str().unwrap(),
            );
            if status.get("state").and_then(Json::str) == Some("done") {
                assert!(status.get("reply").and_then(Json::str).unwrap().contains(reply));
                break;
            }
            assert!(t.elapsed() < Duration::from_secs(30), "{status}");
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(h.gui.shell.agent.input, "My unsent draft");
        assert!(h.gui.shell.agent.entries.iter().any(|e| matches!(e, forge_render::shell::agent::Entry::User {text, ..} if text.starts_with("[Terminal agent request"))));
        drop(h);
        assert!(chat_bridge::call(&window, Json::obj(vec![("method", "info".into())])).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
}
#[test]
fn queued_messages_do_not_follow_a_window_to_another_project() {
    let (mut h, root, window) = setup(Kind::Codex, "switch");
    let sent = request(&window, "send", "message", "Make a change here");
    std::fs::create_dir_all(root.join("another")).unwrap();
    h.gui.agent.project_dir = Some(root.join("another"));
    h.step();
    let status = request(
        &window,
        "status",
        "request",
        sent.get("request").unwrap().str().unwrap(),
    );
    assert_eq!(status.get("state").and_then(Json::str), Some("failed"));
    assert!(h.gui.shell.agent.entries.is_empty());
    drop(h);
    std::fs::remove_dir_all(root).unwrap();
}

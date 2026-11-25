//! The page's side of the agent bridge (src/agent/bridge.rs): the bridge's stream read into the
//! same transcript the desktop's Claude Code gives, its files put in the page's filesystem and
//! told as changes, its failures as the turn's end. (The page itself: web/tests/agent.cjs.)

use forge_agent::json::Json;
use forge_render::agent::Event;
use forge_render::agent::bridge::{MOUNT, reply, take_line, why};
use forge_render::agent::claude::Parser;

#[test]
fn a_file_the_bridge_sends_is_loaded_and_told_with_what_it_held() {
    let mut p = Parser::default();
    let mut loaded = vec![];
    let mut load = |path: &str, text: &str, etag: &str| {
        loaded.push((path.to_string(), text.to_string(), etag.to_string()));
        Some("param extrude1.d = 12\n".to_string())
    };
    let line = r#"{"type":"forge_file","path":"parts/plate.fg","text":"param extrude1.d = 14\n","etag":"\"ab\""}"#;
    let evs = take_line(&mut p, line, &mut load);
    assert_eq!(
        evs,
        vec![Event::FileChanged {
            path: format!("{MOUNT}/parts/plate.fg"),
            before: Some("param extrude1.d = 12\n".into()),
            text: "param extrude1.d = 14\n".into(),
        }]
    );
    assert_eq!(loaded, [(format!("{MOUNT}/parts/plate.fg"), "param extrude1.d = 14\n".into(), "\"ab\"".into())]);
}

#[test]
fn claude_codes_own_lines_read_as_on_the_desktop() {
    let mut p = Parser::default();
    let mut load = |_: &str, _: &str, _: &str| None;
    let ask = r#"{"type":"control_request","request_id":"req-1","request":{"subtype":"can_use_tool","tool_name":"Edit","input":{"file_path":"plate.fg","old_string":"12","new_string":"14"},"permission_suggestions":[],"tool_use_id":"t-1"}}"#;
    let evs = take_line(&mut p, ask, &mut load);
    assert!(matches!(&evs[..], [Event::Permission { id, tool_use, .. }] if id == "req-1" && tool_use == "t-1"));
    let done = take_line(&mut p, r#"{"type":"result","is_error":true,"result":"Claude Code stopped: boom"}"#, &mut load);
    assert_eq!(
        done,
        vec![Event::Done {
            error: Some("Claude Code stopped: boom".into()),
            stopped: false
        }]
    );
    // the bridge's own word that the session ended badly ends the turn with it
    let gone = take_line(&mut p, r#"{"type":"forge_bridge","error":"bypass refused"}"#, &mut load);
    assert_eq!(
        gone,
        vec![Event::Done {
            error: Some("bypass refused".into()),
            stopped: false
        }]
    );
    // not JSON, or a file with no path: nothing
    assert!(take_line(&mut p, "<script>", &mut load).is_empty());
    assert!(take_line(&mut p, r#"{"type":"forge_file","text":"x"}"#, &mut load).is_empty());
}

#[test]
fn a_calls_reply_and_why_it_failed() {
    let (s, j) = reply(r#"{"status":503,"body":"{\"error\":\"Claude Code isn't installed on this machine.\",\"fix\":\"curl\"}"}"#);
    assert_eq!(s, 503);
    assert_eq!(why(s, &j), "Claude Code isn't installed on this machine.");
    assert_eq!(j.get("fix").and_then(Json::str), Some("curl"));
    let (s, j) = reply(r#"{"status":0,"body":"not json"}"#);
    assert_eq!(why(s, &j), "forgecad web isn't reachable: is it still running?");
    assert_eq!(why(409, &Json::Null), "forgecad web said HTTP 409");
}

//! `forgecad web DIR --agent`: the page's agent is the person's own Claude Code, run here
//! (research/agent-tab/bridge-threat-model.md). Against a stand-in `claude` (a shell script,
//! FORGECAD_CLAUDE), each of the model's rules: the token, Origin, Host, JSON only, fields, modes,
//! paths, loopback only, the command line, the environment, one owner, the stream, answers and
//! stop reaching the process, the files it changed reaching the page, and a process that dies.
#![cfg(unix)]

use forge_agent::json::Json;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

const PART: &str = "param extrude1.d = 12\n";

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("forgecad-bridge-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("proj")).unwrap();
    d
}

/// A stand-in `claude`: writes its arguments and environment, then for each message asks to
/// Edit plate.fg, edits it if allowed, and ends the turn. Every line it's sent after the first
/// message goes to heard.jsonl. `MODE` is what its init says.
fn fake(dir: &Path, mode: &str, die: bool) -> PathBuf {
    let d = dir.display();
    let ask = r#"{"type":"control_request","request_id":"req-1","request":{"subtype":"can_use_tool","tool_name":"Edit","input":{"file_path":"plate.fg","old_string":"12","new_string":"14"},"permission_suggestions":[],"tool_use_id":"t-1"}}"#;
    let tool = r#"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"t-1","content":"ok"}]},"parent_tool_use_id":null}"#;
    let then = if die {
        "echo 'boom: the model went away' >&2; exit 3".to_string()
    } else {
        format!(
            "case \"$reply\" in *'\"allow\"'*) sed -i 's/12/14/' plate.fg;; esac\necho '{tool}'\necho '{{\"type\":\"result\",\"subtype\":\"success\",\"session_id\":\"s\"}}'"
        )
    };
    let script = dir.join("claude");
    std::fs::write(
        &script,
        format!(
            "#!/bin/sh\nprintf '%s\\n' \"$@\" > '{d}/args.txt'\necho \"CLAUDECODE=${{CLAUDECODE:-unset}} PWD=$PWD\" > '{d}/env.txt'\n\
             echo '{{\"type\":\"system\",\"subtype\":\"init\",\"session_id\":\"s\",\"model\":\"m\",\"permissionMode\":\"{mode}\"}}'\n\
             while read line; do\n  case \"$line\" in *'\"type\":\"user\"'*) echo \"$line\" >> '{d}/said.jsonl';; *) echo \"$line\" >> '{d}/heard.jsonl'; continue;; esac\n\
               echo '{ask}'\n  read reply\n  echo \"$reply\" >> '{d}/heard.jsonl'\n  {then}\ndone\n"
        ),
    )
    .unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    script
}

struct Server {
    child: Child,
    port: u16,
    token: String,
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn serve(dir: &Path, claude: &Path) -> Server {
    let mut child = Command::new(env!("CARGO_BIN_EXE_forgecad"))
        .args([
            "web",
            dir.join("proj").to_str().unwrap(),
            "--port",
            "0",
            "--agent",
        ])
        .env("FORGECAD_CLAUDE", claude)
        .env("CLAUDECODE", "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut line = String::new();
    BufReader::new(child.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    // "forgecad web: serving DIR at http://localhost:PORT/#agent=TOKEN"
    let url = line.split(" at ").nth(1).unwrap().trim().to_string();
    let port = url
        .split(':')
        .nth(2)
        .unwrap()
        .split('/')
        .next()
        .unwrap()
        .parse()
        .unwrap();
    let token = url.split("#agent=").nth(1).unwrap().to_string();
    assert_eq!(token.len(), 32, "128 bits, as hex");
    Server { child, port, token }
}

struct Reply {
    status: u16,
    head: String,
    body: String,
}

fn request(port: u16, method: &str, path: &str, headers: &[(&str, &str)], body: &str) -> Reply {
    let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
    let mut q = format!(
        "{method} {path} HTTP/1.1\r\nConnection: close\r\nContent-Length: {}\r\n",
        body.len()
    );
    if !headers.iter().any(|h| h.0 == "Host") {
        q.push_str(&format!("Host: localhost:{port}\r\n"));
    }
    for (k, v) in headers {
        q.push_str(&format!("{k}: {v}\r\n"));
    }
    q.push_str("\r\n");
    q.push_str(body);
    s.write_all(q.as_bytes()).unwrap();
    let mut all = String::new();
    s.read_to_string(&mut all).unwrap();
    let (head, body) = all.split_once("\r\n\r\n").unwrap_or((&all, ""));
    Reply {
        status: head.split(' ').nth(1).unwrap().parse().unwrap(),
        head: head.to_string(),
        body: body.to_string(),
    }
}

fn post(srv: &Server, call: &str, body: &str, client: &str) -> Reply {
    let auth = format!("Bearer {}", srv.token);
    request(
        srv.port,
        "POST",
        &format!("/api/agent/{call}"),
        &[
            ("Authorization", &auth),
            ("Content-Type", "application/json"),
            ("X-Forge-Client", client),
        ],
        body,
    )
}

/// The event stream's lines, read on a thread as they come.
fn events(srv: &Server, from: usize, client: &str) -> std::sync::mpsc::Receiver<String> {
    let (tx, rx) = std::sync::mpsc::channel();
    let mut s = TcpStream::connect(("127.0.0.1", srv.port)).unwrap();
    write!(
        s,
        "GET /api/agent/events?from={from} HTTP/1.1\r\nHost: localhost:{}\r\nAuthorization: Bearer {}\r\nX-Forge-Client: {client}\r\n\r\n",
        srv.port, srv.token
    )
    .unwrap();
    std::thread::spawn(move || {
        let r = BufReader::new(s);
        for l in r.lines().map_while(Result::ok) {
            let l = l.trim().to_string();
            // chunk sizes and blank keep-alives aside: the JSON lines
            if l.starts_with('{') && tx.send(l).is_err() {
                return;
            }
        }
    });
    rx
}

fn until(rx: &std::sync::mpsc::Receiver<String>, what: impl Fn(&Json) -> bool) -> Json {
    let t = Instant::now();
    while t.elapsed() < Duration::from_secs(30) {
        if let Ok(l) = rx.recv_timeout(Duration::from_millis(200))
            && let Ok(j) = Json::parse(&l)
            && what(&j)
        {
            return j;
        }
    }
    panic!("not said in 30 s");
}

fn ty(j: &Json) -> &str {
    j.get("type").and_then(Json::str).unwrap_or("")
}

fn wait_file(p: &Path, what: impl Fn(&str) -> bool) -> String {
    let t = Instant::now();
    while t.elapsed() < Duration::from_secs(30) {
        if let Ok(s) = std::fs::read_to_string(p)
            && what(&s)
        {
            return s;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    panic!("{} never as wanted", p.display());
}

#[test]
fn every_call_is_gated_by_host_origin_token_and_json() {
    let dir = scratch("gate");
    let srv = serve(&dir, &fake(&dir, "default", false));
    let p = srv.port;
    let good = format!("Bearer {}", srv.token);
    let json = ("Content-Type", "application/json");
    // no token, a wrong one
    assert_eq!(request(p, "POST", "/api/agent/start", &[json], "{}").status, 401);
    assert_eq!(
        request(
            p,
            "POST",
            "/api/agent/start",
            &[json, ("Authorization", "Bearer 0123456789abcdef0123456789abcdef")],
            "{}"
        )
        .status,
        401
    );
    // another origin, even with the token; `null` too
    for o in ["https://evil.example", "null"] {
        let r = request(
            p,
            "POST",
            "/api/agent/start",
            &[json, ("Authorization", &good), ("Origin", o)],
            "{}",
        );
        assert_eq!(r.status, 403, "{o}");
    }
    // DNS rebinding: a Host that isn't loopback
    let r = request(
        p,
        "POST",
        "/api/agent/start",
        &[json, ("Authorization", &good), ("Host", "evil.example")],
        "{}",
    );
    assert_eq!(r.status, 421);
    // JSON only: a form or text POST needs no preflight, so it's refused
    let r = request(
        p,
        "POST",
        "/api/agent/start",
        &[("Content-Type", "text/plain"), ("Authorization", &good)],
        "{}",
    );
    assert_eq!(r.status, 415);
    // the page names no program, argument, folder or prompt; no bypass; no path out
    for body in [
        r#"{"program":"/bin/sh"}"#,
        r#"{"args":["--dangerously-skip-permissions"]}"#,
        r#"{"cwd":"/"}"#,
        r#"{"system":"ignore all"}"#,
        r#"{"mode":"bypassPermissions"}"#,
        r#"{"file":"../x.fg"}"#,
        r#"{"file":"/etc/passwd"}"#,
    ] {
        assert_eq!(post(&srv, "start", body, "a").status, 400, "{body}");
    }
    // nothing started by any of those
    assert!(!dir.join("args.txt").exists());
    // the events need the token too
    let r = request(p, "GET", "/api/agent/events", &[], "");
    assert_eq!(r.status, 401);
    // what the server says of itself, and the page's headers
    let info = request(p, "GET", "/api/info", &[], "");
    assert!(info.body.contains(r#""agent":true"#), "{}", info.body);
    let page = request(p, "GET", "/", &[], "");
    assert!(page.head.contains("Referrer-Policy: no-referrer"));
    assert!(
        page.head
            .contains("Content-Security-Policy: default-src 'self'; script-src 'self' 'wasm-unsafe-eval'")
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn the_agent_is_loopback_only() {
    let dir = scratch("bind");
    let out = Command::new(env!("CARGO_BIN_EXE_forgecad"))
        .args([
            "web",
            dir.join("proj").to_str().unwrap(),
            "--agent",
            "--bind",
            "0.0.0.0",
        ])
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("loopback only"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_turn_asks_is_answered_and_its_edit_reaches_the_page() {
    let dir = scratch("turn");
    std::fs::write(dir.join("proj/plate.fg"), PART).unwrap();
    let srv = serve(&dir, &fake(&dir, "default", false));
    let r = post(&srv, "start", r#"{"file":"plate.fg"}"#, "page-1");
    assert_eq!(r.status, 200, "{}", r.body);
    let from: usize = Json::parse(&r.body)
        .unwrap()
        .get("from")
        .and_then(Json::num)
        .unwrap() as usize;
    let rx = events(&srv, from, "page-1");
    let init = until(&rx, |j| ty(j) == "system");
    assert_eq!(init.get("permissionMode").and_then(Json::str), Some("default"));
    // the command line: Claude Code's own permissions, asked through the app; nothing forced
    let args = std::fs::read_to_string(dir.join("args.txt")).unwrap();
    assert!(args.contains("--permission-prompt-tool\nstdio\n"));
    assert!(
        !args.contains("--permission-mode"),
        "the person's own defaultMode stands"
    );
    assert!(
        !args.contains("--disallowedTools") && !args.contains("--allowedTools"),
        "no added rules"
    );
    assert!(!args.contains("--dangerously-skip-permissions"));
    assert!(
        args.contains("\"mcp\",\"--root\""),
        "the forgecad MCP server on the project"
    );
    // its environment: the parent's, less the marker of a Claude Code session above it; in DIR
    let env = std::fs::read_to_string(dir.join("env.txt")).unwrap();
    assert!(env.contains("CLAUDECODE=unset"), "{env}");
    assert!(env.contains(&format!(
        "PWD={}",
        std::fs::canonicalize(dir.join("proj")).unwrap().display()
    )));
    // another page can't take it while this one reads
    assert_eq!(post(&srv, "send", r#"{"text":"hi"}"#, "page-2").status, 409);
    assert_eq!(
        post(&srv, "send", r#"{"text":"make it thicker"}"#, "page-1").status,
        200
    );
    let ask = until(&rx, |j| ty(j) == "control_request");
    assert_eq!(ask.get("request_id").and_then(Json::str), Some("req-1"));
    // an answer to a question that isn't waiting
    assert_eq!(
        post(&srv, "answer", r#"{"id":"nope","answer":"allow"}"#, "page-1").status,
        404
    );
    assert_eq!(
        post(&srv, "answer", r#"{"id":"req-1","answer":"maybe"}"#, "page-1").status,
        400
    );
    assert_eq!(
        post(&srv, "answer", r#"{"id":"req-1","answer":"allow"}"#, "page-1").status,
        200
    );
    // the file it changed, to the page, then the turn's end
    let f = until(&rx, |j| ty(j) == "forge_file");
    assert_eq!(f.get("path").and_then(Json::str), Some("plate.fg"));
    assert_eq!(f.get("text").and_then(Json::str), Some("param extrude1.d = 14\n"));
    assert!(
        f.get("etag")
            .and_then(Json::str)
            .is_some_and(|e| e.starts_with('"'))
    );
    until(&rx, |j| ty(j) == "result");
    let heard = wait_file(&dir.join("heard.jsonl"), |s| s.contains("control_response"));
    let reply = Json::parse(heard.lines().next().unwrap()).unwrap();
    let inner = reply.get("response").and_then(|r| r.get("response")).unwrap();
    assert_eq!(inner.get("behavior").and_then(Json::str), Some("allow"));
    assert_eq!(
        inner
            .get("updatedInput")
            .and_then(|i| i.get("new_string"))
            .and_then(Json::str),
        Some("14")
    );
    // the mode and stop reach the process as control requests
    assert_eq!(
        post(&srv, "mode", r#"{"mode":"acceptEdits"}"#, "page-1").status,
        200
    );
    assert_eq!(
        post(&srv, "mode", r#"{"mode":"bypassPermissions"}"#, "page-1").status,
        400
    );
    assert_eq!(post(&srv, "stop", "{}", "page-1").status, 200);
    let heard = wait_file(&dir.join("heard.jsonl"), |s| s.contains("interrupt"));
    assert!(
        heard.contains(r#""subtype":"set_permission_mode","mode":"acceptEdits""#),
        "{heard}"
    );
    // a page that reconnects gets the lines again from where it asks
    let again = events(&srv, from, "page-1");
    until(&again, |j| ty(j) == "forge_file");
    drop(srv);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_chosen_mode_goes_on_the_command_line_and_a_bypass_from_the_persons_settings_runs() {
    let dir = scratch("mode");
    std::fs::write(dir.join("proj/plate.fg"), PART).unwrap();
    let srv = serve(&dir, &fake(&dir, "bypassPermissions", false));
    assert_eq!(post(&srv, "start", r#"{"mode":"plan"}"#, "p").status, 200);
    let rx = events(&srv, 0, "p");
    // case 1 (same origin) honours the person's own bypass, as a terminal does; the page says so
    let init = until(&rx, |j| ty(j) == "system");
    assert_eq!(
        init.get("permissionMode").and_then(Json::str),
        Some("bypassPermissions")
    );
    let args = wait_file(&dir.join("args.txt"), |s| !s.is_empty());
    assert!(args.contains("--permission-mode\nplan\n"));
    drop(srv);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_messages_pictures_go_to_claude_code_as_image_blocks_and_nothing_else_does() {
    let dir = scratch("pictures");
    std::fs::write(dir.join("proj/plate.fg"), PART).unwrap();
    let srv = serve(&dir, &fake(&dir, "default", false));
    assert_eq!(post(&srv, "start", "{}", "p").status, 200);
    // refused: a type the models don't read (an SVG can carry script), data that isn't base64,
    // a field that isn't one, not a list
    for bad in [
        r#"{"text":"x","pictures":[{"media":"image/svg+xml","data":"PHN2Zz4="}]}"#,
        r#"{"text":"x","pictures":[{"media":"image/png","data":"a\"b"}]}"#,
        r#"{"text":"x","pictures":[{"media":"image/png","data":"iVBO","url":"file:///etc/passwd"}]}"#,
        r#"{"text":"x","pictures":{"media":"image/png","data":"iVBO"}}"#,
        r#"{"text":"x","picture":"iVBO"}"#,
    ] {
        let r = post(&srv, "send", bad, "p");
        assert_eq!(r.status, 400, "{bad}: {}", r.body);
    }
    let r = post(
        &srv,
        "send",
        r#"{"text":"what's this?","pictures":[{"media":"image/png","data":"iVBORw0K"},{"media":"image/jpeg","data":"/9j/4A=="}]}"#,
        "p",
    );
    assert_eq!(r.status, 200, "{}", r.body);
    let said = wait_file(&dir.join("said.jsonl"), |s| !s.is_empty());
    let msg = Json::parse(said.lines().next().unwrap()).unwrap();
    let content = msg.get("message").and_then(|m| m.get("content")).unwrap();
    let Json::Arr(blocks) = content else {
        panic!("{content:?}")
    };
    let shape: Vec<(String, String)> = blocks
        .iter()
        .map(|b| {
            (
                b.get("type").and_then(Json::str).unwrap().to_string(),
                b.get("source")
                    .and_then(|s| s.get("media_type"))
                    .and_then(Json::str)
                    .unwrap_or("")
                    .to_string(),
            )
        })
        .collect();
    assert_eq!(
        shape,
        [
            ("image".into(), "image/png".into()),
            ("image".into(), "image/jpeg".into()),
            ("text".into(), String::new())
        ]
    );
}

#[test]
fn a_process_that_dies_mid_turn_ends_the_turn_with_why() {
    let dir = scratch("die");
    std::fs::write(dir.join("proj/plate.fg"), PART).unwrap();
    let srv = serve(&dir, &fake(&dir, "default", true));
    assert_eq!(post(&srv, "start", "{}", "p").status, 200);
    let rx = events(&srv, 0, "p");
    assert_eq!(post(&srv, "send", r#"{"text":"go"}"#, "p").status, 200);
    until(&rx, |j| ty(j) == "control_request");
    assert_eq!(
        post(&srv, "answer", r#"{"id":"req-1","answer":"deny"}"#, "p").status,
        200
    );
    let end = until(&rx, |j| ty(j) == "result");
    assert_eq!(end.get("is_error"), Some(&Json::Bool(true)));
    assert!(
        end.get("result")
            .and_then(Json::str)
            .is_some_and(|r| r.contains("boom: the model went away")),
        "{end:?}"
    );
    // nothing to send to now
    assert_eq!(post(&srv, "send", r#"{"text":"again"}"#, "p").status, 409);
    drop(srv);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn claude_code_not_installed_says_how_to_install_it() {
    let dir = scratch("missing");
    let srv = serve(&dir, Path::new("/nonexistent/claude"));
    let r = post(&srv, "start", "{}", "p");
    assert_eq!(r.status, 503);
    assert!(
        r.body.contains("isn't installed") && r.body.contains("claude.ai/install.sh"),
        "{}",
        r.body
    );
    drop(srv);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn the_bridges_own_code_names_no_credential() {
    // T7: Claude Code loads its own login; the bridge never reads, sets or forwards one
    let src = include_str!("../src/agent_bridge.rs").to_lowercase();
    for word in [
        ".claude/",
        "credentials",
        "anthropic_api_key",
        "anthropic_auth_token",
        "oauth",
        "keychain",
    ] {
        assert!(!src.contains(word), "agent_bridge.rs names `{word}`");
    }
}

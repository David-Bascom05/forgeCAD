//! Claude Code, headless: `claude -p` reading and writing stream-json, one process per chat kept
//! open across turns (resumed by its session id if it ends), David's own login and settings.

use super::{Backend, Event, Out, Watch, clean_env};
use crate::shell::agent::tool_summary;
use forge_agent::json::Json;
use std::collections::HashSet;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

pub const INSTALL: &str = "curl -fsSL https://claude.ai/install.sh | bash";
pub const LOGIN: &str = "claude auth login";

/// The stream-json lines a turn writes, as the transcript's events.
#[derive(Default)]
pub struct Parser {
    pub session: Option<String>,
    pub model: Option<String>,
    /// Messages whose text came a piece at a time: their whole is said already.
    streamed: HashSet<String>,
    tools: HashSet<String>,
    /// This turn already said why it failed.
    failed: bool,
}

fn get<'a>(j: &'a Json, k: &str) -> Option<&'a Json> {
    j.get(k)
}

fn str_of<'a>(j: &'a Json, k: &str) -> &'a str {
    j.get(k).and_then(Json::str).unwrap_or("")
}

fn arr(j: Option<&Json>) -> &[Json] {
    match j {
        Some(Json::Arr(a)) => a,
        _ => &[],
    }
}

/// A tool result's text: a string, or text blocks.
fn content_text(c: Option<&Json>) -> String {
    match c {
        Some(Json::Str(s)) => s.clone(),
        Some(Json::Arr(a)) => a
            .iter()
            .filter_map(|b| b.get("text").and_then(Json::str))
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    }
}

impl Parser {
    pub fn line(&mut self, line: &str) -> Vec<Event> {
        let Ok(d) = Json::parse(line.trim()) else {
            return vec![];
        };
        let mut out = Vec::new();
        // a subagent's own work stays inside its tool row
        if get(&d, "parent_tool_use_id").is_some_and(|p| *p != Json::Null) {
            return out;
        }
        match str_of(&d, "type") {
            "system" if str_of(&d, "subtype") == "init" => {
                self.session = Some(str_of(&d, "session_id").to_string());
                self.model = Some(str_of(&d, "model").to_string());
            }
            "stream_event" => {
                let Some(e) = get(&d, "event") else { return out };
                match str_of(e, "type") {
                    "content_block_start" => {
                        let b = get(e, "content_block");
                        if let Some(b) = b
                            && str_of(b, "type") == "tool_use"
                        {
                            let id = str_of(b, "id").to_string();
                            self.tools.insert(id.clone());
                            out.push(Event::ToolStart {
                                id,
                                name: str_of(b, "name").to_string(),
                            });
                        }
                    }
                    "content_block_delta" => {
                        if let Some(delta) = get(e, "delta")
                            && str_of(delta, "type") == "text_delta"
                        {
                            self.streamed.insert(str_of(&d, "api_message_id").to_string());
                            out.push(Event::Text(str_of(delta, "text").to_string()));
                        }
                    }
                    _ => {}
                }
            }
            "assistant" => {
                let Some(m) = get(&d, "message") else { return out };
                let text = arr(get(m, "content"))
                    .iter()
                    .filter_map(|b| b.get("text").and_then(Json::str))
                    .collect::<Vec<_>>()
                    .join("\n");
                match str_of(&d, "error") {
                    "" => {}
                    "authentication_failed" => {
                        self.failed = true;
                        out.push(Event::Problem {
                            text: "Claude Code isn't signed in.".into(),
                            fix: Some(LOGIN.into()),
                        });
                        return out;
                    }
                    _ => {
                        self.failed = true;
                        out.push(Event::Note { text, error: true });
                        return out;
                    }
                }
                let streamed = self.streamed.contains(str_of(m, "id"));
                for b in arr(get(m, "content")) {
                    match str_of(b, "type") {
                        "text" if !streamed => out.push(Event::Text(str_of(b, "text").to_string())),
                        "tool_use" => {
                            let id = str_of(b, "id").to_string();
                            let name = str_of(b, "name").to_string();
                            if self.tools.insert(id.clone()) {
                                out.push(Event::ToolStart {
                                    id: id.clone(),
                                    name: name.clone(),
                                });
                            }
                            let (summary, input) = tool_summary(&name, b.get("input").unwrap_or(&Json::Null));
                            out.push(Event::ToolInput { id, summary, input });
                        }
                        _ => {}
                    }
                }
            }
            "user" => {
                let Some(m) = get(&d, "message") else { return out };
                for b in arr(get(m, "content")) {
                    if str_of(b, "type") == "tool_result" {
                        out.push(Event::ToolDone {
                            id: str_of(b, "tool_use_id").to_string(),
                            output: content_text(b.get("content")),
                            error: b.get("is_error") == Some(&Json::Bool(true)),
                        });
                    }
                }
            }
            "result" => {
                let stopped = str_of(&d, "terminal_reason") == "aborted_streaming";
                let is_error = d.get("is_error") == Some(&Json::Bool(true));
                let error = (is_error && !stopped && !self.failed).then(|| {
                    let r = str_of(&d, "result");
                    if r.is_empty() {
                        arr(get(&d, "errors"))
                            .iter()
                            .filter_map(Json::str)
                            .collect::<Vec<_>>()
                            .join("; ")
                    } else {
                        r.to_string()
                    }
                });
                if let Some(s) = d.get("session_id").and_then(Json::str) {
                    self.session = Some(s.to_string());
                }
                self.failed = false;
                out.push(Event::Done { error, stopped });
            }
            _ => {}
        }
        out
    }
}

pub struct Config {
    pub program: String,
    /// `--model`; empty for Claude Code's default.
    pub model: String,
    pub file: PathBuf,
    pub system: String,
    /// Where it runs: a folder of ForgeCAD's own, never the part's.
    pub cwd: PathBuf,
    /// The `--mcp-config` of the app's own tools; none: no MCP server at all.
    pub mcp: Option<String>,
}

/// Where Claude Code runs: empty and ForgeCAD's, so a downloaded project's `.claude/`,
/// `.mcp.json` and `CLAUDE.md` never load; the part's folder comes in by `--add-dir`.
pub fn clean_cwd() -> PathBuf {
    let base = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/state")))
        .unwrap_or_else(std::env::temp_dir);
    base.join("forgecad").join("agent")
}

pub struct ClaudeCode {
    cfg: Config,
    out: Out,
    child: Option<Child>,
    stdin: Option<ChildStdin>,
    /// The chat's session, for `--resume` when the process has to start again.
    pub session: Arc<Mutex<Option<String>>>,
    alive: Arc<Mutex<bool>>,
    /// A turn's running: the process ending now ends it with an error.
    busy: Arc<AtomicBool>,
    stops: u64,
}

impl ClaudeCode {
    pub fn new(cfg: Config, out: Out, session: Option<String>) -> ClaudeCode {
        ClaudeCode {
            cfg,
            out,
            child: None,
            stdin: None,
            session: Arc::new(Mutex::new(session)),
            alive: Arc::new(Mutex::new(false)),
            busy: Arc::new(AtomicBool::new(false)),
            stops: 0,
        }
    }

    /// The command line, as run. Only David's user settings load (no project or local
    /// settings, so no project hooks), only ForgeCAD's MCP server, no Bash, and in `dontAsk`
    /// mode only what's allowed here runs: the folder's files, and the app's tools (fg among them).
    pub fn args(&self) -> Vec<String> {
        let dir = self
            .cfg
            .file
            .parent()
            .map(|d| d.display().to_string())
            .unwrap_or_default();
        let mut a: Vec<String> = [
            "-p",
            "--output-format",
            "stream-json",
            "--input-format",
            "stream-json",
            "--verbose",
            "--include-partial-messages",
            "--setting-sources",
            "user",
            "--strict-mcp-config",
            "--permission-mode",
            "dontAsk",
            "--tools",
            "Read,Edit,Write",
            "--add-dir",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        a.push(dir.clone());
        a.push("--mcp-config".into());
        a.push(
            self.cfg
                .mcp
                .clone()
                .unwrap_or_else(|| r#"{"mcpServers":{}}"#.into()),
        );
        a.push("--allowedTools".into());
        if self.cfg.mcp.is_some() {
            a.push(format!("mcp__{}", super::mcp::SERVER));
        }
        // `//` starts an absolute path in a permission rule
        for t in ["Read", "Edit", "Write"] {
            a.push(format!("{t}(/{dir}/**)"));
        }
        a.push("--append-system-prompt".into());
        a.push(self.cfg.system.clone());
        if !self.cfg.model.trim().is_empty() {
            a.push("--model".into());
            a.push(self.cfg.model.trim().into());
        }
        if let Some(s) = self.session.lock().expect("no panics while held").clone() {
            a.push("--resume".into());
            a.push(s);
        }
        a
    }

    fn spawn(&mut self) -> Result<(), Event> {
        let dir = self.cfg.cwd.clone();
        if let Err(e) = std::fs::create_dir_all(&dir) {
            return Err(Event::Problem {
                text: format!("No folder for the agent to run in: {}: {e}", dir.display()),
                fix: None,
            });
        }
        let mut c = Command::new(&self.cfg.program);
        c.args(self.args())
            .current_dir(&dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        clean_env(&mut c);
        let mut child = c.spawn().map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                Event::Problem {
                    text: "Claude Code isn't installed.".into(),
                    fix: Some(INSTALL.into()),
                }
            } else {
                Event::Problem {
                    text: format!("Claude Code didn't start: {e}"),
                    fix: None,
                }
            }
        })?;
        let stdout = child.stdout.take().expect("piped");
        let stderr = child.stderr.take().expect("piped");
        self.stdin = child.stdin.take();
        self.child = Some(child);
        *self.alive.lock().expect("no panics while held") = true;
        let said = Arc::new(Mutex::new(String::new()));
        let said2 = said.clone();
        std::thread::Builder::new()
            .name("agent-claude-err".into())
            .spawn(move || {
                for l in BufReader::new(stderr).lines().map_while(Result::ok) {
                    let mut s = said2.lock().expect("no panics while held");
                    s.push_str(&l);
                    s.push('\n');
                    if s.len() > 4000 {
                        let cut = s.len() - 4000;
                        let cut = (cut..s.len()).find(|k| s.is_char_boundary(*k)).unwrap_or(0);
                        s.drain(..cut);
                    }
                }
            })
            .expect("a thread");
        let (out, session, alive, busy, file) = (
            self.out.clone(),
            self.session.clone(),
            self.alive.clone(),
            self.busy.clone(),
            self.cfg.file.clone(),
        );
        std::thread::Builder::new()
            .name("agent-claude".into())
            .spawn(move || {
                let mut parser = Parser::default();
                let mut watch = Watch::new(file.parent().unwrap_or(std::path::Path::new(".")));
                for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                    for e in parser.line(&line) {
                        let after = matches!(e, Event::ToolDone { .. } | Event::Done { .. });
                        if after {
                            for c in watch.check() {
                                out.send(c);
                            }
                        }
                        if matches!(e, Event::Done { .. } | Event::Problem { .. }) {
                            busy.store(false, Ordering::SeqCst);
                        }
                        out.send(e);
                    }
                    if let Some(s) = &parser.session {
                        *session.lock().expect("no panics while held") = Some(s.clone());
                    }
                }
                *alive.lock().expect("no panics while held") = false;
                for c in watch.check() {
                    out.send(c);
                }
                if !busy.swap(false, Ordering::SeqCst) {
                    return;
                }
                // ended mid-turn: say why, from what it wrote on stderr
                std::thread::sleep(std::time::Duration::from_millis(50));
                let err = said.lock().expect("no panics while held").trim().to_string();
                let last = err.lines().last().unwrap_or("").to_string();
                out.send(Event::Done {
                    error: Some(if last.is_empty() {
                        "Claude Code stopped".into()
                    } else {
                        format!("Claude Code stopped: {last}")
                    }),
                    stopped: false,
                });
            })
            .expect("a thread");
        Ok(())
    }

    fn write(&mut self, j: &Json) -> bool {
        let Some(w) = &mut self.stdin else { return false };
        let mut s = String::new();
        j.write(&mut s);
        s.push('\n');
        w.write_all(s.as_bytes()).and_then(|_| w.flush()).is_ok()
    }
}

impl Backend for ClaudeCode {
    fn send(&mut self, prompt: String) {
        let msg = Json::obj(vec![
            ("type", "user".into()),
            (
                "message",
                Json::obj(vec![("role", "user".into()), ("content", prompt.into())]),
            ),
        ]);
        self.busy.store(true, Ordering::SeqCst);
        let alive = *self.alive.lock().expect("no panics while held");
        if (!alive || !self.write(&msg))
            && let Err(e) = self.spawn().map(|_| self.write(&msg))
        {
            self.busy.store(false, Ordering::SeqCst);
            self.out.send(e);
        }
    }

    fn stop(&mut self) {
        self.stops += 1;
        let req = Json::obj(vec![
            ("type", "control_request".into()),
            ("request_id", format!("stop-{}", self.stops).into()),
            ("request", Json::obj(vec![("subtype", "interrupt".into())])),
        ]);
        if !self.write(&req) {
            self.out.send(Event::Done {
                error: None,
                stopped: true,
            });
        }
    }
}

impl Drop for ClaudeCode {
    fn drop(&mut self) {
        self.stdin = None;
        if let Some(mut c) = self.child.take() {
            let _ = c.kill();
            std::thread::spawn(move || c.wait());
        }
    }
}
